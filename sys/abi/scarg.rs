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
//! `SCARG`: what `syscalls.master`'s argument types read out of a 64-bit argument register.
//!
//! `syscallarg(t)` (`<sys/syscallargs.h>`) is a union of the `register_t` the trap saved and
//! a `t` at its low end, and both machines are little-endian, so the C's `SCARG(uap, x)` reads
//! an `int` as the register's low 32 bits with their sign, a `u_int` as the low 32 bits
//! unsigned, a pointer or a `size_t` as the 64 bits, an `off_t` as the 64 bits signed. One
//! function per family, named after what `syscalls.master` writes, each the one cast the C's
//! union amounts to: the high word of a 32-bit argument is ignored as the C ignores it, never
//! refused (`i32::try_from` would answer an error the C never gives). The two narrowings are
//! the crate's only `#[expect(clippy::cast_*)]`s, with the table's size in `sysent.rs`.

use hal::syscall::Register;

/// `int`, `pid_t`, `dev_t`, `clockid_t`: the register's low 32 bits, signed.
#[inline(always)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "SCARG: syscallarg(int) is the register's low 32 bits, the C's truncation"
)]
pub fn int(r: Register) -> i32 {
    r as i32
}

/// `u_int`, `mode_t`, `uid_t`, `gid_t`, `id_t`, `uint32_t`: the register's low 32 bits,
/// unsigned (chown(2)'s `-1` is `0xffff_ffff` here, as the C's `uid != (uid_t)-1` compares it).
#[inline(always)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "SCARG: syscallarg(u_int) is the register's low 32 bits, unsigned"
)]
pub fn u_int(r: Register) -> u32 {
    r as u32
}

/// A pointer argument: the 64 bits as a user address (0 for `NULL`).
#[inline(always)]
pub fn ptr(r: Register) -> usize {
    r.cast_unsigned()
}

/// `size_t`: the 64 bits as a count.
#[inline(always)]
pub fn size_t(r: Register) -> usize {
    r.cast_unsigned()
}

/// `off_t`: the 64 bits, signed.
#[inline(always)]
pub fn off_t(r: Register) -> i64 {
    r as i64
}

/// `u_long`, `uint64_t`: the 64 bits, unsigned (ioctl(2)'s `com`; kbind(2)'s cookie, which the
/// C's `sys_kbind` reads as `syscallarg(uint64_t)`, `uvm_mmap.c`).
#[inline(always)]
pub fn u_long(r: Register) -> u64 {
    r.cast_unsigned() as u64
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// The C's union on a little-endian LP64 machine: the 32-bit types see the low word only,
    /// the 64-bit ones the whole register.
    #[test]
    fn each_family_reads_the_register_as_the_c_union_does() {
        let ints: [(Register, i32); 5] = [
            (0, 0),
            (-1, -1),
            (0x1_0000_0002, 2),
            (0x7fff_ffff_8000_0000, i32::MIN),
            (-0x8000_0000, i32::MIN),
        ];
        for (r, want) in ints {
            assert_eq!(int(r), want, "int({r:#x})");
        }
        let uints: [(Register, u32); 4] = [
            (0, 0),
            (-1, 0xffff_ffff),
            (0x1_0000_0002, 2),
            (0x7fff_ffff_8000_0000, 0x8000_0000),
        ];
        for (r, want) in uints {
            assert_eq!(u_int(r), want, "u_int({r:#x})");
        }
        assert_eq!(ptr(0), 0);
        assert_eq!(ptr(-1), usize::MAX);
        assert_eq!(ptr(0x1_0000_0002), 0x1_0000_0002);
        assert_eq!(size_t(Register::MIN), 1 << 63);
        assert_eq!(size_t(Register::MAX), usize::MAX >> 1);
        assert_eq!(off_t(-1), -1);
        assert_eq!(off_t(Register::MAX), i64::MAX);
        assert_eq!(off_t(Register::MIN), i64::MIN);
        assert_eq!(u_long(-1), u64::MAX);
        assert_eq!(u_long(0x1_0000_0002), 0x1_0000_0002);
    }
}
/* </TESTS> */
