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
 * Copyright (c) 1990, 1991, 1992, 1993
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
 *	@(#)bpf_filter.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The filter machine of `bpf(4)`: `net/bpf_filter.c`'s interpreter `_bpf_lfilter`, which
//! runs a program over a packet, and `bpf_validate`, the check a program passes before the
//! kernel installs it (`BIOCSETF`, `BIOCSETWF`, `BIOCSETFNR`).
//!
//! Upstream: sys/net/bpf_filter.c @ 3ce1f3f79392
//! LZ: sys/net/bpf_filter.rs@3c62ede68802
//!
//! The kernel compiles only `_bpf_lfilter` and `bpf_validate`; the packet is read through
//! the [`BpfOps`] its type implements (`bpf.c`'s `bpf_mbuf_ops`: `kernel::net::bpf`'s
//! `BpfPkt`, an mbuf chain behind an optional header). Every bound of the C is kept where
//! the C has it: a short packet, a scratch word past [`BPF_MEMWORDS`], a jump out of the
//! program, a division or modulus by zero and an unknown opcode reject the packet (0); a
//! shift by 32 or more yields 0; falling off the end rejects.
//!
//! ## Deviations
//! - The program is a slice (`pc` and `pc_len` in one), `None` for the C's NULL program
//!   (accept all). A NULL-terminated walk (`pc_len` 0 in the C, used only by userland's
//!   `bpf_filter`) does not exist: an empty slice runs no instruction and rejects, which the
//!   kernel never installs (`bpf_validate` refuses it, as the C's does).
//! - The instruction pointer is an index; a jump that leaves the program ends the walk as
//!   the C's `pc < pcend` test does. The C's `from` counter (which a jump does not advance)
//!   is kept, with its checks.
//! - `bpf_validate` takes the program as a slice and answers `bool` (the C's `int` 0/1).
//! - The userland half (`bpf_mem_*`, `bpf_filter`, `bpf_lfilter`, `_bpf_filter`, compiled
//!   without `_KERNEL` for libpcap) is not ported; the host tests read packets through a
//!   slice of their own.
//!
//! ## Redesign
//! - LZ's `inherited` module of `bsd`, moved into the kernel (N6.3c1) with its shape; `bsd`'s
//!   copy goes with the device (`kernel::net::bpf`).
//! - `struct bpf_ops`, LZ's `BpfOps<P>` of three function pointers over `&P` -> the trait
//!   [`BpfOps`] the packet type implements, and `_bpf_lfilter` generic over it: the C's table
//!   of loads, called directly (the mbuf loads inline into the interpreter where the C and LZ
//!   call through pointers). One implementer in the kernel (`BpfPkt`) and the tests' slice.
//! - `bpf_validate`'s read of `bpf_maxbufsize` (the C's global, `[a]`, read once per load
//!   instruction) -> the bound as a parameter, which `bpf_setf` reads once from the atomic
//!   `net.bpf.maxbufsize` (`kernel::net::bpf`): the module holds no state, and a program is
//!   checked against one value of the bound, not against two if a `sysctl` writes it midway.

use hal::sys::net_bpf::{
    BPF_A, BPF_ABS, BPF_ADD, BPF_ALU, BPF_AND, BPF_B, BPF_DIV, BPF_H, BPF_IMM, BPF_IND, BPF_JA,
    BPF_JEQ, BPF_JGE, BPF_JGT, BPF_JMP, BPF_JSET, BPF_K, BPF_LD, BPF_LDX, BPF_LEN, BPF_LSH,
    BPF_MAXINSNS, BPF_MEM, BPF_MEMWORDS, BPF_MISC, BPF_MOD, BPF_MSH, BPF_MUL, BPF_NEG, BPF_OR,
    BPF_RET, BPF_RND, BPF_RSH, BPF_ST, BPF_STX, BPF_SUB, BPF_TAX, BPF_TXA, BPF_W, BPF_X, BPF_XOR,
    BpfInsn, bpf_class, bpf_mode, bpf_op, bpf_src,
};

/// `struct bpf_ops`: the loads the interpreter makes against a packet. Each reads the word,
/// half-word or byte at offset `k` in network byte order, `None` when the packet is too
/// short (the C's `*err = 1`).
pub trait BpfOps {
    /// `ldw`: the 32-bit word at `k`.
    fn ldw(&self, k: u32) -> Option<u32>;
    /// `ldh`: the 16-bit half-word at `k`.
    fn ldh(&self, k: u32) -> Option<u32>;
    /// `ldb`: the byte at `k`.
    fn ldb(&self, k: u32) -> Option<u32>;
}

#[cfg(test)]
std::thread_local! {
    /// The instructions `_bpf_lfilter` ran on this thread: the property test's step count.
    static STEPS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// `_bpf_lfilter`: runs the filter program `pc` on the packet `pkt`, whose original length
/// is `wirelen`. Returns how many bytes of the packet to keep: 0 rejects it, `u32::MAX` (the
/// C's `(u_int)-1`) keeps all of it.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one switch over every opcode, kept as one match"
)]
pub fn _bpf_lfilter<P: BpfOps + ?Sized>(pc: Option<&[BpfInsn]>, pkt: &P, wirelen: u32) -> u32 {
    let Some(prog) = pc else {
        // No program accepts everything.
        return u32::MAX;
    };
    let Ok(pc_len) = u32::try_from(prog.len()) else {
        return 0;
    };
    if pc_len > BPF_MAXINSNS {
        return 0;
    }

    let mut a: u32 = 0;
    let mut x: u32 = 0;
    let mut from: u32 = 1;
    let mut mem = [0u32; BPF_MEMWORDS];

    // Whether a conditional jump of `k` from the `from`th instruction run stays inside the
    // program (the C's `from + k >= pc_len` refusal).
    let inside = |from: u32, k: u32| from.wrapping_add(k) < pc_len;

    let mut i = 0usize;
    while let Some(insn) = prog.get(i) {
        #[cfg(test)]
        STEPS.with(|s| s.set(s.get() + 1));
        let k = insn.k;
        match insn.code {
            c if c == BPF_RET | BPF_K => return k,

            c if c == BPF_RET | BPF_A => return a,

            c if c == BPF_LD | BPF_W | BPF_ABS => match pkt.ldw(k) {
                Some(v) => a = v,
                None => return 0,
            },

            c if c == BPF_LD | BPF_H | BPF_ABS => match pkt.ldh(k) {
                Some(v) => a = v,
                None => return 0,
            },

            c if c == BPF_LD | BPF_B | BPF_ABS => match pkt.ldb(k) {
                Some(v) => a = v,
                None => return 0,
            },

            c if c == BPF_LD | BPF_W | BPF_IND => match pkt.ldw(x.wrapping_add(k)) {
                Some(v) => a = v,
                None => return 0,
            },

            c if c == BPF_LD | BPF_H | BPF_IND => match pkt.ldh(x.wrapping_add(k)) {
                Some(v) => a = v,
                None => return 0,
            },

            c if c == BPF_LD | BPF_B | BPF_IND => match pkt.ldb(x.wrapping_add(k)) {
                Some(v) => a = v,
                None => return 0,
            },

            c if c == BPF_LDX | BPF_B | BPF_MSH => match pkt.ldb(k) {
                Some(v) => x = (v & 0xf) << 2,
                None => return 0,
            },

            c if c == BPF_LD | BPF_W | BPF_LEN => a = wirelen,

            c if c == BPF_LDX | BPF_W | BPF_LEN => x = wirelen,

            c if c == BPF_LD | BPF_W | BPF_RND => a = hal::random::arc4random(),

            c if c == BPF_LD | BPF_IMM => a = k,

            c if c == BPF_LDX | BPF_IMM => x = k,

            c if c == BPF_LD | BPF_MEM => match mem.get(k as usize) {
                Some(&v) => a = v,
                None => return 0,
            },

            c if c == BPF_LDX | BPF_MEM => match mem.get(k as usize) {
                Some(&v) => x = v,
                None => return 0,
            },

            BPF_ST => match mem.get_mut(k as usize) {
                Some(slot) => *slot = a,
                None => return 0,
            },

            BPF_STX => match mem.get_mut(k as usize) {
                Some(slot) => *slot = x,
                None => return 0,
            },

            c if c == BPF_JMP | BPF_JA => {
                if from.checked_add(k).is_none() || !inside(from, k) {
                    return 0;
                }
                i += k as usize;
            }

            c if bpf_class(c) == BPF_JMP && is_cond_jump(c) => {
                let operand = if bpf_src(c) == BPF_X { x } else { k };
                let taken = match bpf_op(c) {
                    BPF_JGT => a > operand,
                    BPF_JGE => a >= operand,
                    BPF_JEQ => a == operand,
                    _ => a & operand != 0,
                };
                let off = u32::from(if taken { insn.jt } else { insn.jf });
                if !inside(from, off) {
                    return 0;
                }
                i += off as usize;
            }

            c if c == BPF_ALU | BPF_ADD | BPF_X => a = a.wrapping_add(x),

            c if c == BPF_ALU | BPF_SUB | BPF_X => a = a.wrapping_sub(x),

            c if c == BPF_ALU | BPF_MUL | BPF_X => a = a.wrapping_mul(x),

            c if c == BPF_ALU | BPF_DIV | BPF_X => match a.checked_div(x) {
                Some(q) => a = q,
                None => return 0,
            },

            c if c == BPF_ALU | BPF_MOD | BPF_X => match a.checked_rem(x) {
                Some(r) => a = r,
                None => return 0,
            },

            c if c == BPF_ALU | BPF_AND | BPF_X => a &= x,

            c if c == BPF_ALU | BPF_OR | BPF_X => a |= x,

            c if c == BPF_ALU | BPF_XOR | BPF_X => a ^= x,

            c if c == BPF_ALU | BPF_LSH | BPF_X => a = a.checked_shl(x).unwrap_or(0),

            c if c == BPF_ALU | BPF_RSH | BPF_X => a = a.checked_shr(x).unwrap_or(0),

            c if c == BPF_ALU | BPF_ADD | BPF_K => a = a.wrapping_add(k),

            c if c == BPF_ALU | BPF_SUB | BPF_K => a = a.wrapping_sub(k),

            c if c == BPF_ALU | BPF_MUL | BPF_K => a = a.wrapping_mul(k),

            c if c == BPF_ALU | BPF_DIV | BPF_K => match a.checked_div(k) {
                Some(q) => a = q,
                None => return 0,
            },

            c if c == BPF_ALU | BPF_MOD | BPF_K => match a.checked_rem(k) {
                Some(r) => a = r,
                None => return 0,
            },

            c if c == BPF_ALU | BPF_AND | BPF_K => a &= k,

            c if c == BPF_ALU | BPF_OR | BPF_K => a |= k,

            c if c == BPF_ALU | BPF_XOR | BPF_K => a ^= k,

            c if c == BPF_ALU | BPF_LSH | BPF_K => a = a.checked_shl(k).unwrap_or(0),

            c if c == BPF_ALU | BPF_RSH | BPF_K => a = a.checked_shr(k).unwrap_or(0),

            c if c == BPF_ALU | BPF_NEG => a = a.wrapping_neg(),

            c if c == BPF_MISC | BPF_TAX => x = a,

            c if c == BPF_MISC | BPF_TXA => a = x,

            _ => return 0,
        }
        i += 1;
        from = from.wrapping_add(1);
    }
    0
}

/// The eight conditional jumps: `BPF_JGT`, `BPF_JGE`, `BPF_JEQ` and `BPF_JSET`, each with a
/// `BPF_K` or a `BPF_X` operand (the C's eight `case` labels).
const fn is_cond_jump(code: u16) -> bool {
    let k_or_x = code & !BPF_X;
    k_or_x == BPF_JMP | BPF_JGT
        || k_or_x == BPF_JMP | BPF_JGE
        || k_or_x == BPF_JMP | BPF_JEQ
        || k_or_x == BPF_JMP | BPF_JSET
}

/// `bpf_validate`: whether `f` is a program the kernel runs. Every jump goes forward to an
/// instruction of the program, every scratch word exists, every packet offset is below
/// `maxbufsize` (`net.bpf.maxbufsize`; the exact length is checked as the program runs),
/// no constant shift reaches 32 and no constant divisor is 0, every opcode is one the
/// interpreter knows, and the last instruction returns. A program that fails any of these
/// could otherwise make the interpreter read or jump where it must not.
pub fn bpf_validate(f: &[BpfInsn], maxbufsize: u32) -> bool {
    let Ok(len) = u32::try_from(f.len()) else {
        return false;
    };
    if !(1..=BPF_MAXINSNS).contains(&len) {
        return false;
    }

    for (from, p) in (1u32..).zip(f) {
        let code = p.code;
        match bpf_class(code) {
            BPF_RET if code == BPF_RET | BPF_K || code == BPF_RET | BPF_A => {}
            // Every load addresses something that can exist.
            BPF_LD | BPF_LDX if is_load(code) => match bpf_mode(code) {
                BPF_IMM | BPF_LEN | BPF_RND => {}
                // The packet's real length is only known when the program runs.
                BPF_ABS | BPF_IND | BPF_MSH => {
                    if p.k >= maxbufsize {
                        return false;
                    }
                }
                BPF_MEM => {
                    if p.k as usize >= BPF_MEMWORDS {
                        return false;
                    }
                }
                _ => return false,
            },
            BPF_ST | BPF_STX if code == BPF_ST || code == BPF_STX => {
                if p.k as usize >= BPF_MEMWORDS {
                    return false;
                }
            }
            // Jumps go forward, to an instruction of the program (`from` is the index of the
            // one after the jump).
            BPF_JMP if code == BPF_JMP | BPF_JA || is_cond_jump(code) => match bpf_op(code) {
                BPF_JA => match from.checked_add(p.k) {
                    Some(to) if to < len => {}
                    _ => return false,
                },
                BPF_JEQ | BPF_JGT | BPF_JGE | BPF_JSET => {
                    if from + u32::from(p.jt) >= len || from + u32::from(p.jf) >= len {
                        return false;
                    }
                }
                _ => return false,
            },
            BPF_ALU if is_alu(code) => match bpf_op(code) {
                BPF_ADD | BPF_SUB | BPF_MUL | BPF_OR | BPF_XOR | BPF_AND | BPF_NEG => {}
                // A constant shift stays below 32 bits.
                BPF_LSH | BPF_RSH => {
                    if bpf_src(code) == BPF_K && p.k > 31 {
                        return false;
                    }
                }
                // A constant divisor is not 0.
                BPF_DIV | BPF_MOD => {
                    if bpf_src(code) == BPF_K && p.k == 0 {
                        return false;
                    }
                }
                _ => return false,
            },
            BPF_MISC if code == BPF_MISC | BPF_TAX || code == BPF_MISC | BPF_TXA => {}
            _ => return false,
        }
    }
    f.last().is_some_and(|last| bpf_class(last.code) == BPF_RET)
}

/// The fourteen load instructions `bpf_validate` accepts (the C's `case` labels).
fn is_load(code: u16) -> bool {
    [
        BPF_LD | BPF_W | BPF_ABS,
        BPF_LD | BPF_H | BPF_ABS,
        BPF_LD | BPF_B | BPF_ABS,
        BPF_LD | BPF_W | BPF_IND,
        BPF_LD | BPF_H | BPF_IND,
        BPF_LD | BPF_B | BPF_IND,
        BPF_LDX | BPF_B | BPF_MSH,
        BPF_LD | BPF_W | BPF_LEN,
        BPF_LDX | BPF_W | BPF_LEN,
        BPF_LD | BPF_W | BPF_RND,
        BPF_LD | BPF_IMM,
        BPF_LDX | BPF_IMM,
        BPF_LD | BPF_MEM,
        BPF_LDX | BPF_MEM,
    ]
    .contains(&code)
}

/// The twenty-one ALU instructions `bpf_validate` accepts (the C's `case` labels): the ten
/// binary operations with a `BPF_X` or a `BPF_K` operand, and `BPF_NEG`.
const fn is_alu(code: u16) -> bool {
    if code == BPF_ALU | BPF_NEG {
        return true;
    }
    let op = code & !BPF_X;
    op == BPF_ALU | BPF_ADD
        || op == BPF_ALU | BPF_SUB
        || op == BPF_ALU | BPF_MUL
        || op == BPF_ALU | BPF_DIV
        || op == BPF_ALU | BPF_MOD
        || op == BPF_ALU | BPF_AND
        || op == BPF_ALU | BPF_OR
        || op == BPF_ALU | BPF_XOR
        || op == BPF_ALU | BPF_LSH
        || op == BPF_ALU | BPF_RSH
}

/// The instructions `_bpf_lfilter` ran on this thread since the last call (tests).
#[cfg(test)]
pub(crate) fn steps_taken() -> u64 {
    STEPS.with(|s| s.replace(0))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use hal::sys::net_bpf::{BPF_MAXBUFSIZE, bpf_jump, bpf_stmt};

    use super::*;

    /// The default bound `bpf_setf` passes (`net.bpf.maxbufsize`).
    const MAXBUF: u32 = BPF_MAXBUFSIZE as u32;

    /// A packet in one linear buffer: userland's `bpf_mem_ops`, for the tests.
    struct MemPkt<'a>(&'a [u8]);

    impl BpfOps for MemPkt<'_> {
        fn ldw(&self, k: u32) -> Option<u32> {
            let b = self.0.get(k as usize..)?.first_chunk::<4>()?;
            Some(u32::from_be_bytes(*b))
        }
        fn ldh(&self, k: u32) -> Option<u32> {
            let b = self.0.get(k as usize..)?.first_chunk::<2>()?;
            Some(u32::from(u16::from_be_bytes(*b)))
        }
        fn ldb(&self, k: u32) -> Option<u32> {
            self.0.get(k as usize).map(|&b| u32::from(b))
        }
    }

    /// `bpf_filter(pc, pkt, wirelen, buflen)` with the whole buffer captured.
    fn run(prog: &[BpfInsn], pkt: &[u8]) -> u32 {
        crate::testutil::install();
        _bpf_lfilter(Some(prog), &MemPkt(pkt), pkt.len() as u32)
    }

    /// `tcpdump -d 'ip and udp and dst port 53'` on Ethernet.
    const UDP_DNS: [BpfInsn; 11] = [
        bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 12),
        bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 0x800, 0, 8),
        bpf_stmt(BPF_LD | BPF_B | BPF_ABS, 23),
        bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 17, 0, 6),
        bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 20),
        bpf_jump(BPF_JMP | BPF_JSET | BPF_K, 0x1fff, 4, 0),
        bpf_stmt(BPF_LDX | BPF_B | BPF_MSH, 14),
        bpf_stmt(BPF_LD | BPF_H | BPF_IND, 16),
        bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 53, 0, 1),
        bpf_stmt(BPF_RET | BPF_K, 262144),
        bpf_stmt(BPF_RET | BPF_K, 0),
    ];

    /// An Ethernet frame carrying IPv4 (IHL 5 + `opts` words of options) and UDP to `dport`.
    fn udp_frame(dport: u16, opts: usize, frag: u16) -> Vec<u8> {
        let mut f = vec![0u8; 14];
        f[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        let mut ip = vec![0u8; 20 + 4 * opts];
        ip[0] = 0x45 + opts as u8;
        ip[6..8].copy_from_slice(&frag.to_be_bytes());
        ip[9] = 17;
        f.extend_from_slice(&ip);
        let mut udp = [0u8; 8];
        udp[0..2].copy_from_slice(&1234u16.to_be_bytes());
        udp[2..4].copy_from_slice(&dport.to_be_bytes());
        f.extend_from_slice(&udp);
        f
    }

    #[test]
    fn canned_program_matches_dns() {
        assert_eq!(run(&UDP_DNS, &udp_frame(53, 0, 0)), 262144);
        assert_eq!(
            run(&UDP_DNS, &udp_frame(53, 2, 0)),
            262144,
            "ldxb 4*([14]&0xf)"
        );
        assert_eq!(run(&UDP_DNS, &udp_frame(80, 0, 0)), 0);
        assert_eq!(
            run(&UDP_DNS, &udp_frame(53, 0, 0x0005)),
            0,
            "a later fragment"
        );
        // ARP.
        let mut arp = udp_frame(53, 0, 0);
        arp[12..14].copy_from_slice(&0x0806u16.to_be_bytes());
        assert_eq!(run(&UDP_DNS, &arp), 0);
        // A truncated capture: the load past the end rejects.
        assert_eq!(run(&UDP_DNS, &udp_frame(53, 0, 0)[..36]), 0);
        assert!(bpf_validate(&UDP_DNS, MAXBUF));
    }

    #[test]
    fn no_program_accepts_all() {
        assert_eq!(_bpf_lfilter(None, &MemPkt(&[]), 0), u32::MAX);
        assert_eq!(run(&[], &[1, 2, 3]), 0, "an empty program runs nothing");
    }

    #[test]
    fn alu_scratch_and_len() {
        let prog = [
            bpf_stmt(BPF_LD | BPF_W | BPF_LEN, 0),
            bpf_stmt(BPF_ALU | BPF_MUL | BPF_K, 3),
            bpf_stmt(BPF_ST, 5),
            bpf_stmt(BPF_LDX | BPF_IMM, 4),
            bpf_stmt(BPF_LD | BPF_MEM, 5),
            bpf_stmt(BPF_ALU | BPF_SUB | BPF_X, 0),
            bpf_stmt(BPF_ALU | BPF_RSH | BPF_K, 1),
            bpf_stmt(BPF_RET | BPF_A, 0),
        ];
        // (10 * 3 - 4) >> 1
        assert_eq!(run(&prog, &[0; 10]), 13);
        assert!(bpf_validate(&prog, MAXBUF));

        let neg = [
            bpf_stmt(BPF_LD | BPF_IMM, 1),
            bpf_stmt(BPF_ALU | BPF_NEG, 0),
            bpf_stmt(BPF_MISC | BPF_TAX, 0),
            bpf_stmt(BPF_LD | BPF_IMM, 7),
            bpf_stmt(BPF_MISC | BPF_TXA, 0),
            bpf_stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(run(&neg, &[]), u32::MAX);

        let modk = [
            bpf_stmt(BPF_LD | BPF_IMM, 10),
            bpf_stmt(BPF_ALU | BPF_MOD | BPF_K, 4),
            bpf_stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(run(&modk, &[]), 2);
    }

    #[test]
    fn walk_ends_off_the_program() {
        // An unchecked program whose jump leaves it rejects, as does falling off the end.
        let out = [
            bpf_jump(BPF_JMP | BPF_JA, 5, 0, 0),
            bpf_stmt(BPF_RET | BPF_K, 1),
        ];
        assert_eq!(run(&out, &[]), 0);
        assert!(!bpf_validate(&out, MAXBUF));
        let fall = [bpf_stmt(BPF_LD | BPF_IMM, 1)];
        assert_eq!(run(&fall, &[]), 0);
        assert!(!bpf_validate(&fall, MAXBUF));
        let bad = [bpf_stmt(0xff, 0), bpf_stmt(BPF_RET | BPF_K, 1)];
        assert_eq!(run(&bad, &[]), 0);
        assert!(!bpf_validate(&bad, MAXBUF));
        let mem = [bpf_stmt(BPF_LD | BPF_MEM, 16), bpf_stmt(BPF_RET | BPF_K, 1)];
        assert_eq!(run(&mem, &[]), 0);
        assert!(!bpf_validate(&mem, MAXBUF));
    }

    /// A packet whose byte `i` is `i + 1`, `n` bytes long.
    fn counting(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i + 1) as u8).collect()
    }

    /// `ret a` after `body`: the accumulator is the answer.
    fn ret_a(body: &[BpfInsn]) -> Vec<BpfInsn> {
        let mut p = body.to_vec();
        p.push(bpf_stmt(BPF_RET | BPF_A, 0));
        p
    }

    #[test]
    fn the_interpreter_keeps_each_bound_of_the_c() {
        let ldx = |k| bpf_stmt(BPF_LDX | BPF_IMM, k);
        let imm = |k| bpf_stmt(BPF_LD | BPF_IMM, k);
        let one = bpf_stmt(BPF_RET | BPF_K, 1);
        // (what, program, packet length, the C's answer). Packets count up from 1.
        let table: Vec<(&str, Vec<BpfInsn>, usize, u32)> = vec![
            // Every load on a packet long enough, and one byte short of it.
            (
                "ldw abs",
                ret_a(&[bpf_stmt(BPF_LD | BPF_W | BPF_ABS, 2)]),
                6,
                0x0304_0506,
            ),
            (
                "ldw abs short",
                ret_a(&[bpf_stmt(BPF_LD | BPF_W | BPF_ABS, 2)]),
                5,
                0,
            ),
            (
                "ldh abs",
                ret_a(&[bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 3)]),
                5,
                0x0405,
            ),
            (
                "ldh abs short",
                ret_a(&[bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 3)]),
                4,
                0,
            ),
            (
                "ldb abs",
                ret_a(&[bpf_stmt(BPF_LD | BPF_B | BPF_ABS, 4)]),
                5,
                5,
            ),
            (
                "ldb abs short",
                ret_a(&[bpf_stmt(BPF_LD | BPF_B | BPF_ABS, 4)]),
                4,
                0,
            ),
            (
                "ldw ind",
                ret_a(&[ldx(1), bpf_stmt(BPF_LD | BPF_W | BPF_IND, 1)]),
                6,
                0x0304_0506,
            ),
            (
                "ldw ind short",
                ret_a(&[ldx(1), bpf_stmt(BPF_LD | BPF_W | BPF_IND, 1)]),
                5,
                0,
            ),
            (
                "ldh ind",
                ret_a(&[ldx(2), bpf_stmt(BPF_LD | BPF_H | BPF_IND, 1)]),
                5,
                0x0405,
            ),
            (
                "ldh ind short",
                ret_a(&[ldx(2), bpf_stmt(BPF_LD | BPF_H | BPF_IND, 1)]),
                4,
                0,
            ),
            (
                "ldb ind",
                ret_a(&[ldx(3), bpf_stmt(BPF_LD | BPF_B | BPF_IND, 1)]),
                5,
                5,
            ),
            (
                "ldb ind short",
                ret_a(&[ldx(3), bpf_stmt(BPF_LD | BPF_B | BPF_IND, 1)]),
                4,
                0,
            ),
            // X + k wraps as the C's u_int32_t does, and the wrapped offset is bounded.
            (
                "ind wraps",
                ret_a(&[ldx(u32::MAX), bpf_stmt(BPF_LD | BPF_B | BPF_IND, 2)]),
                2,
                2,
            ),
            (
                "msh",
                vec![
                    bpf_stmt(BPF_LDX | BPF_B | BPF_MSH, 0),
                    bpf_stmt(BPF_MISC | BPF_TXA, 0),
                    bpf_stmt(BPF_RET | BPF_A, 0),
                ],
                1,
                4,
            ),
            (
                "msh short",
                vec![bpf_stmt(BPF_LDX | BPF_B | BPF_MSH, 1), one],
                1,
                0,
            ),
            ("len", ret_a(&[bpf_stmt(BPF_LD | BPF_W | BPF_LEN, 0)]), 9, 9),
            // Scratch memory: 16 words, zeroed.
            ("mem 15", ret_a(&[bpf_stmt(BPF_LD | BPF_MEM, 15)]), 0, 0),
            ("ld mem 16", vec![bpf_stmt(BPF_LD | BPF_MEM, 16), one], 0, 0),
            (
                "ldx mem 16",
                vec![bpf_stmt(BPF_LDX | BPF_MEM, 16), one],
                0,
                0,
            ),
            ("st 16", vec![bpf_stmt(BPF_ST, 16), one], 0, 0),
            ("stx 16", vec![bpf_stmt(BPF_STX, 16), one], 0, 0),
            (
                "st ld 15",
                ret_a(&[
                    imm(7),
                    bpf_stmt(BPF_ST, 15),
                    imm(0),
                    bpf_stmt(BPF_LD | BPF_MEM, 15),
                ]),
                0,
                7,
            ),
            // Jumps: to the last instruction, past it, wrapping.
            (
                "ja last",
                vec![
                    bpf_jump(BPF_JMP | BPF_JA, 1, 0, 0),
                    bpf_stmt(BPF_RET | BPF_K, 2),
                    one,
                ],
                0,
                1,
            ),
            (
                "ja past",
                vec![
                    bpf_jump(BPF_JMP | BPF_JA, 2, 0, 0),
                    bpf_stmt(BPF_RET | BPF_K, 2),
                    one,
                ],
                0,
                0,
            ),
            (
                "ja wraps",
                vec![bpf_jump(BPF_JMP | BPF_JA, u32::MAX, 0, 0), one],
                0,
                0,
            ),
            // Division and modulus by zero reject; by one do not.
            (
                "div x 0",
                vec![imm(9), bpf_stmt(BPF_ALU | BPF_DIV | BPF_X, 0), one],
                0,
                0,
            ),
            (
                "mod x 0",
                vec![imm(9), bpf_stmt(BPF_ALU | BPF_MOD | BPF_X, 0), one],
                0,
                0,
            ),
            (
                "div k 0",
                vec![imm(9), bpf_stmt(BPF_ALU | BPF_DIV | BPF_K, 0), one],
                0,
                0,
            ),
            (
                "mod k 0",
                vec![imm(9), bpf_stmt(BPF_ALU | BPF_MOD | BPF_K, 0), one],
                0,
                0,
            ),
            (
                "div x 2",
                ret_a(&[imm(9), ldx(2), bpf_stmt(BPF_ALU | BPF_DIV | BPF_X, 0)]),
                0,
                4,
            ),
            (
                "mod k 4",
                ret_a(&[imm(9), bpf_stmt(BPF_ALU | BPF_MOD | BPF_K, 4)]),
                0,
                1,
            ),
            // Shifts by 32 or more are 0; by 31 they shift.
            (
                "lsh x 32",
                ret_a(&[imm(1), ldx(32), bpf_stmt(BPF_ALU | BPF_LSH | BPF_X, 0)]),
                0,
                0,
            ),
            (
                "lsh x 31",
                ret_a(&[imm(1), ldx(31), bpf_stmt(BPF_ALU | BPF_LSH | BPF_X, 0)]),
                0,
                1 << 31,
            ),
            (
                "rsh x 32",
                ret_a(&[
                    imm(u32::MAX),
                    ldx(32),
                    bpf_stmt(BPF_ALU | BPF_RSH | BPF_X, 0),
                ]),
                0,
                0,
            ),
            (
                "rsh x 31",
                ret_a(&[
                    imm(u32::MAX),
                    ldx(31),
                    bpf_stmt(BPF_ALU | BPF_RSH | BPF_X, 0),
                ]),
                0,
                1,
            ),
            (
                "lsh k 32",
                ret_a(&[imm(1), bpf_stmt(BPF_ALU | BPF_LSH | BPF_K, 32)]),
                0,
                0,
            ),
            (
                "rsh k 40",
                ret_a(&[imm(u32::MAX), bpf_stmt(BPF_ALU | BPF_RSH | BPF_K, 40)]),
                0,
                0,
            ),
            // Arithmetic wraps as u_int32_t.
            (
                "add wraps",
                ret_a(&[imm(u32::MAX), bpf_stmt(BPF_ALU | BPF_ADD | BPF_K, 2)]),
                0,
                1,
            ),
            (
                "sub wraps",
                ret_a(&[imm(0), bpf_stmt(BPF_ALU | BPF_SUB | BPF_K, 1)]),
                0,
                u32::MAX,
            ),
            (
                "mul wraps",
                ret_a(&[imm(0x8000_0000), bpf_stmt(BPF_ALU | BPF_MUL | BPF_K, 2)]),
                0,
                0,
            ),
            (
                "neg",
                ret_a(&[imm(2), bpf_stmt(BPF_ALU | BPF_NEG, 0)]),
                0,
                u32::MAX - 1,
            ),
            (
                "and or xor",
                ret_a(&[
                    imm(0xf0),
                    bpf_stmt(BPF_ALU | BPF_AND | BPF_K, 0x3c),
                    bpf_stmt(BPF_ALU | BPF_OR | BPF_K, 1),
                    bpf_stmt(BPF_ALU | BPF_XOR | BPF_K, 0xff),
                ]),
                0,
                0xce,
            ),
            // Unknown opcodes and the end of the program reject.
            ("undefined", vec![bpf_stmt(0x0f, 0), one], 0, 0),
            (
                "ja x",
                vec![bpf_stmt(BPF_JMP | BPF_JA | BPF_X, 0), one],
                0,
                0,
            ),
            (
                "neg x",
                vec![bpf_stmt(BPF_ALU | BPF_NEG | BPF_X, 0), one],
                0,
                0,
            ),
            ("falls off", vec![imm(1)], 0, 0),
            ("ret k", vec![bpf_stmt(BPF_RET | BPF_K, 77)], 0, 77),
            ("ret x", vec![ldx(5), bpf_stmt(BPF_RET | BPF_X, 0)], 0, 0),
        ];
        for (what, prog, len, want) in &table {
            assert_eq!(run(prog, &counting(*len)), *want, "{what}");
        }
        // A program past BPF_MAXINSNS is refused before it runs.
        let long = vec![bpf_stmt(BPF_RET | BPF_K, 1); BPF_MAXINSNS as usize + 1];
        assert_eq!(run(&long, &[]), 0, "pc_len > BPF_MAXINSNS");
        let max = vec![bpf_stmt(BPF_RET | BPF_K, 1); BPF_MAXINSNS as usize];
        assert_eq!(run(&max, &[]), 1);
    }

    #[test]
    fn each_conditional_jump_stays_inside_the_program() {
        let one = bpf_stmt(BPF_RET | BPF_K, 1);
        let two = bpf_stmt(BPF_RET | BPF_K, 2);
        // A is 1; the operand (k, or X loaded with k) makes each test true, then false.
        let cases = [
            (BPF_JEQ, [(1, true), (2, false)]),
            (BPF_JGT, [(0, true), (1, false)]),
            (BPF_JGE, [(1, true), (2, false)]),
            (BPF_JSET, [(3, true), (2, false)]),
        ];
        for (op, operands) in cases {
            for src in [BPF_K, BPF_X] {
                let code = BPF_JMP | op | src;
                for (k, taken) in operands {
                    let setup = [
                        bpf_stmt(BPF_LD | BPF_IMM, 1),
                        bpf_stmt(BPF_LDX | BPF_IMM, k),
                    ];
                    // jt/jf of 1 reach the last instruction; 2 is past it.
                    for (off, want) in [(1u8, 2), (2u8, 0)] {
                        let (jt, jf) = if taken { (off, 9) } else { (9, off) };
                        let mut p = setup.to_vec();
                        p.push(bpf_jump(code, k, jt, jf));
                        p.push(one);
                        p.push(two);
                        assert_eq!(run(&p, &[]), want, "code {code:#x} k {k} off {off}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_validator_keeps_each_rule_of_the_c() {
        let ret = bpf_stmt(BPF_RET | BPF_K, 0);
        let with = |i: BpfInsn| [i, ret];
        // (what, program, accepted).
        let table: Vec<(&str, Vec<BpfInsn>, bool)> = vec![
            ("empty", vec![], false),
            ("one ret", vec![ret], true),
            ("ret a", vec![bpf_stmt(BPF_RET | BPF_A, 0)], true),
            ("ret x", vec![bpf_stmt(BPF_RET | BPF_X, 0)], false),
            ("512", vec![ret; BPF_MAXINSNS as usize], true),
            ("513", vec![ret; BPF_MAXINSNS as usize + 1], false),
            (
                "last not ret",
                vec![ret, bpf_stmt(BPF_LD | BPF_IMM, 0)],
                false,
            ),
            (
                "abs below bound",
                with(bpf_stmt(BPF_LD | BPF_B | BPF_ABS, MAXBUF - 1)).to_vec(),
                true,
            ),
            (
                "abs at bound",
                with(bpf_stmt(BPF_LD | BPF_W | BPF_ABS, MAXBUF)).to_vec(),
                false,
            ),
            (
                "ind at bound",
                with(bpf_stmt(BPF_LD | BPF_H | BPF_IND, MAXBUF)).to_vec(),
                false,
            ),
            (
                "msh at bound",
                with(bpf_stmt(BPF_LDX | BPF_B | BPF_MSH, MAXBUF)).to_vec(),
                false,
            ),
            (
                "imm any k",
                with(bpf_stmt(BPF_LD | BPF_IMM, u32::MAX)).to_vec(),
                true,
            ),
            (
                "len",
                with(bpf_stmt(BPF_LDX | BPF_W | BPF_LEN, 0)).to_vec(),
                true,
            ),
            (
                "rnd",
                with(bpf_stmt(BPF_LD | BPF_W | BPF_RND, 0)).to_vec(),
                true,
            ),
            (
                "mem 15",
                with(bpf_stmt(BPF_LDX | BPF_MEM, 15)).to_vec(),
                true,
            ),
            (
                "mem 16",
                with(bpf_stmt(BPF_LD | BPF_MEM, 16)).to_vec(),
                false,
            ),
            ("st 16", with(bpf_stmt(BPF_ST, 16)).to_vec(), false),
            ("stx 16", with(bpf_stmt(BPF_STX, 16)).to_vec(), false),
            (
                "ld h imm",
                with(bpf_stmt(BPF_LD | BPF_H | BPF_IMM, 0)).to_vec(),
                false,
            ),
            (
                "ld b len",
                with(bpf_stmt(BPF_LD | BPF_B | BPF_LEN, 0)).to_vec(),
                false,
            ),
            (
                "ldx w abs",
                with(bpf_stmt(BPF_LDX | BPF_W | BPF_ABS, 0)).to_vec(),
                false,
            ),
            (
                "lsh k 31",
                with(bpf_stmt(BPF_ALU | BPF_LSH | BPF_K, 31)).to_vec(),
                true,
            ),
            (
                "lsh k 32",
                with(bpf_stmt(BPF_ALU | BPF_LSH | BPF_K, 32)).to_vec(),
                false,
            ),
            (
                "rsh k 32",
                with(bpf_stmt(BPF_ALU | BPF_RSH | BPF_K, 32)).to_vec(),
                false,
            ),
            (
                "lsh x any",
                with(bpf_stmt(BPF_ALU | BPF_LSH | BPF_X, 99)).to_vec(),
                true,
            ),
            (
                "div k 0",
                with(bpf_stmt(BPF_ALU | BPF_DIV | BPF_K, 0)).to_vec(),
                false,
            ),
            (
                "mod k 0",
                with(bpf_stmt(BPF_ALU | BPF_MOD | BPF_K, 0)).to_vec(),
                false,
            ),
            (
                "div x",
                with(bpf_stmt(BPF_ALU | BPF_DIV | BPF_X, 0)).to_vec(),
                true,
            ),
            ("neg", with(bpf_stmt(BPF_ALU | BPF_NEG, 0)).to_vec(), true),
            (
                "neg x",
                with(bpf_stmt(BPF_ALU | BPF_NEG | BPF_X, 0)).to_vec(),
                false,
            ),
            (
                "alu 0xb0",
                with(bpf_stmt(BPF_ALU | 0xb0, 0)).to_vec(),
                false,
            ),
            (
                "tax txa",
                vec![
                    bpf_stmt(BPF_MISC | BPF_TAX, 0),
                    bpf_stmt(BPF_MISC | BPF_TXA, 0),
                    ret,
                ],
                true,
            ),
            (
                "misc 0x40",
                with(bpf_stmt(BPF_MISC | 0x40, 0)).to_vec(),
                false,
            ),
            (
                "ja to last",
                with(bpf_jump(BPF_JMP | BPF_JA, 0, 0, 0)).to_vec(),
                true,
            ),
            (
                "ja past",
                with(bpf_jump(BPF_JMP | BPF_JA, 1, 0, 0)).to_vec(),
                false,
            ),
            (
                "ja wraps",
                with(bpf_jump(BPF_JMP | BPF_JA, u32::MAX, 0, 0)).to_vec(),
                false,
            ),
            (
                "ja x",
                with(bpf_jump(BPF_JMP | BPF_JA | BPF_X, 0, 0, 0)).to_vec(),
                false,
            ),
            (
                "jeq to last",
                with(bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 0)).to_vec(),
                true,
            ),
            (
                "jeq jt past",
                with(bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 1, 0)).to_vec(),
                false,
            ),
            (
                "jgt jf past",
                with(bpf_jump(BPF_JMP | BPF_JGT | BPF_X, 0, 0, 1)).to_vec(),
                false,
            ),
            (
                "jge to last",
                with(bpf_jump(BPF_JMP | BPF_JGE | BPF_X, 0, 0, 0)).to_vec(),
                true,
            ),
            (
                "jset jt past",
                with(bpf_jump(BPF_JMP | BPF_JSET | BPF_K, 0, 1, 0)).to_vec(),
                false,
            ),
            (
                "jmp 0x50",
                with(bpf_jump(BPF_JMP | 0x50, 0, 0, 0)).to_vec(),
                false,
            ),
            ("undefined", with(bpf_stmt(0xffff, 0)).to_vec(), false),
        ];
        for (what, prog, ok) in &table {
            assert_eq!(bpf_validate(prog, MAXBUF), *ok, "{what}");
            assert_eq!(
                reference_validate(prog, MAXBUF),
                *ok,
                "the reference on {what}"
            );
        }
        // The bound is the caller's (net.bpf.maxbufsize): a smaller one refuses more.
        let abs = with(bpf_stmt(BPF_LD | BPF_B | BPF_ABS, 100));
        assert!(bpf_validate(&abs, 101) && !bpf_validate(&abs, 100));
    }

    /// `bpf_validate` as the C's switch reads, case label by case label: written apart from
    /// the module's (which groups the labels by class), so the property test compares two
    /// readings of the C.
    fn reference_validate(f: &[BpfInsn], maxbuf: u32) -> bool {
        let len = f.len() as u64;
        if !(1..=u64::from(BPF_MAXINSNS)).contains(&len) {
            return false;
        }
        for (i, p) in f.iter().enumerate() {
            let from = i as u64 + 1;
            let k = u64::from(p.k);
            let ok = match p.code {
                // ret k, ret a
                0x06 | 0x16 => true,
                // ld w/h/b abs, ld w/h/b ind, ldx msh: below the bound
                0x20 | 0x28 | 0x30 | 0x40 | 0x48 | 0x50 | 0xb1 => p.k < maxbuf,
                // ld len, ldx len, ld rnd, ld imm, ldx imm
                0x80 | 0x81 | 0xc0 | 0x00 | 0x01 => true,
                // ld mem, ldx mem, st, stx: a scratch word
                0x60 | 0x61 | 0x02 | 0x03 => k < 16,
                // ja
                0x05 => from + k < len,
                // jeq, jgt, jge, jset with k or x
                0x15 | 0x25 | 0x35 | 0x45 | 0x1d | 0x2d | 0x3d | 0x4d => {
                    from + u64::from(p.jt) < len && from + u64::from(p.jf) < len
                }
                // add, sub, mul, or, and, xor (k and x), div x, mod x, lsh x, rsh x, neg
                0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x44 | 0x4c | 0x54 | 0x5c | 0xa4
                | 0xac | 0x3c | 0x9c | 0x6c | 0x7c | 0x84 => true,
                // div k, mod k: not by 0
                0x34 | 0x94 => p.k != 0,
                // lsh k, rsh k: below 32
                0x64 | 0x74 => p.k <= 31,
                // tax, txa
                0x07 | 0x87 => true,
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        f.last().is_some_and(|l| l.code & 0x07 == 0x06)
    }

    /// xorshift64*: the property test's deterministic source.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// The codes the C's switches name, and a few around them.
    const CODES: [u16; 52] = [
        0x06, 0x16, 0x20, 0x28, 0x30, 0x40, 0x48, 0x50, 0xb1, 0x80, 0x81, 0xc0, 0x00, 0x01, 0x60,
        0x61, 0x02, 0x03, 0x05, 0x15, 0x25, 0x35, 0x45, 0x1d, 0x2d, 0x3d, 0x4d, 0x04, 0x0c, 0x14,
        0x1c, 0x24, 0x2c, 0x34, 0x3c, 0x94, 0x9c, 0x44, 0x4c, 0x54, 0x5c, 0xa4, 0xac, 0x64, 0x6c,
        0x74, 0x7c, 0x84, 0x07, 0x87, 0x0d, 0x8c,
    ];

    /// An operand the bounds care about, or a small one.
    fn operand(r: &mut Rng, maxbuf: u32) -> u32 {
        const EDGES: [u32; 10] = [0, 1, 15, 16, 31, 32, 255, 256, u32::MAX, u32::MAX - 1];
        match r.below(4) {
            0 => EDGES[r.below(EDGES.len() as u64) as usize],
            1 => maxbuf - 1 + r.below(2) as u32,
            _ => r.below(64) as u32,
        }
    }

    fn random_program(r: &mut Rng, maxbuf: u32) -> Vec<BpfInsn> {
        // Short programs half the time, so many pass the validator.
        let n = 1 + if r.below(2) == 0 {
            r.below(8)
        } else {
            r.below(64)
        } as usize;
        let mut p: Vec<BpfInsn> = (0..n)
            .map(|_| {
                let code = if r.below(16) == 0 {
                    r.next() as u16
                } else {
                    CODES[r.below(CODES.len() as u64) as usize]
                };
                BpfInsn {
                    code,
                    jt: r.below(8) as u8,
                    jf: r.below(8) as u8,
                    k: operand(r, maxbuf),
                }
            })
            .collect();
        if r.below(4) != 0 {
            let last = p.len() - 1;
            p[last].code = if r.below(2) == 0 { 0x06 } else { 0x16 };
        }
        p
    }

    #[test]
    fn an_accepted_program_runs_without_panic_in_at_most_its_length_in_steps() {
        crate::testutil::install();
        // A small bound too, so packet offsets meet it.
        for maxbuf in [MAXBUF, 200] {
            let mut r = Rng(0x9e37_79b9_7f4a_7c15 ^ u64::from(maxbuf));
            let (mut accepted, mut ran) = (0u32, 0u32);
            for _ in 0..40_000 {
                let prog = random_program(&mut r, maxbuf);
                let ok = bpf_validate(&prog, maxbuf);
                assert_eq!(ok, reference_validate(&prog, maxbuf), "{prog:?}");
                if !ok {
                    continue;
                }
                accepted += 1;
                for _ in 0..4 {
                    let pkt: Vec<u8> = (0..r.below(257)).map(|_| r.next() as u8).collect();
                    let _ = steps_taken();
                    let _ = _bpf_lfilter(Some(&prog), &MemPkt(&pkt), pkt.len() as u32);
                    let steps = steps_taken();
                    assert!(steps <= prog.len() as u64, "{steps} steps of {prog:?}");
                    ran += 1;
                }
            }
            assert!(
                accepted > 1000,
                "the generator reaches valid programs ({accepted})"
            );
            assert!(ran > 4000);
        }
    }
}
/* </TESTS> */
