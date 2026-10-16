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
//! `UserPtr<T>`, `AbiPod` and `abi_pod!`: the typed `copyin(9)`/`copyout(9)`.
//!
//! The C copies a structure with `copyin(uaddr, &obj, sizeof(obj))`, trusting the type to be
//! plain bytes; LZ marks such types with an `unsafe trait AbiPod` and an `unsafe impl` per
//! type (about 200, `sys/machine/copy.rs`), each an `unsafe` the type's owner must vouch for.
//! Here (decision 17) [`AbiPod`] is a safe trait whose impls only [`abi_pod!`] writes (and the
//! primitives below): the macro checks at compile time that every field listed is exactly of
//! the type listed, that every field type is itself `AbiPod`, and that the fields, in the
//! order listed, tile the structure with no byte of padding, and that they are all its fields
//! (an exhaustive destructuring: a field left out, even a zero-sized one, or a union does not
//! compile). So every byte
//! of a value is initialised, every bit pattern is a valid value, and the one `unsafe` that
//! views a value as bytes lives here. `cargo xtask lz check` refuses an `impl AbiPod` written
//! by hand (the premise the byte views' `SAFETY:` arguments rest on, like `Adapter`'s).
//!
//! A [`UserPtr<T>`] is a user address typed by what it points at: [`UserPtr::copyin`] and
//! [`UserPtr::copyout`] move a whole `T`, the slice forms move `[T]` (`kevent(2)`'s arrays),
//! [`UserPtr::copyinstr`] a NUL-terminated string. The copies themselves are the machine's
//! (`copy.S` with `pcb_onfault`), reached through the [`UserCopy`] bridge (bridges.toml; paid
//! by N4, when the machine code is `hal`'s own). The bridge is a safe trait: it reads and
//! writes the byte slices it is given and nothing else, so no implementation can make this
//! module unsound.
//!
//! **Before registration** (early boot, host tests that register none) there is no user
//! address space: every copy fails with `EFAULT`.
//!
//! ## Redesign
//! - `copyin(uaddr, &obj, sizeof(obj))` -> `UserPtr::<T>::new(uaddr).copyin()`, returning the
//!   value; `copyout(&obj, uaddr, sizeof(obj))` -> `.copyout(&obj)`. The size is the type's,
//!   so it cannot disagree with the object.
//! - The `unsafe impl AbiPod for T {}` of each owner -> `abi_pod!(T { field: Type, ... })`
//!   beside the type, checked by the compiler; the legacy trait of `sys/machine/copy.rs` stays
//!   for its legacy users until each type's owner moves.

use core::marker::PhantomData;
use core::mem::MaybeUninit;

use crate::bridges;
use crate::sys::errno::Errno;

/// The machine's copy routines, as `bsd` registers them (`sys/machine/copy.rs`).
pub trait UserCopy: Sync {
    /// `copyin(uaddr, kbuf, kbuf.len())`: `EFAULT` when the user range is not mapped or not
    /// user space.
    fn copyin(&self, uaddr: usize, kbuf: &mut [u8]) -> Result<(), Errno>;
    /// `copyout(kbuf, uaddr, kbuf.len())`.
    fn copyout(&self, kbuf: &[u8], uaddr: usize) -> Result<(), Errno>;
    /// `copyinstr(uaddr, kbuf, kbuf.len(), &done)`: the bytes copied, the NUL included;
    /// `ENAMETOOLONG` when the string does not fit.
    fn copyinstr(&self, uaddr: usize, kbuf: &mut [u8]) -> Result<usize, Errno>;
    /// `copyoutstr(kbuf, uaddr, kbuf.len(), &done)`: the bytes copied, the NUL included.
    fn copyoutstr(&self, kbuf: &[u8], uaddr: usize) -> Result<usize, Errno>;
}

/// A type the kernel copies to and from userland as the bytes it is in memory: every byte of
/// a value is initialised and every bit pattern is a valid value.
///
/// Implemented only by [`abi_pod!`] (which checks it at compile time) and, here, for the
/// integers and for arrays of `AbiPod` types; a hand-written impl is refused by `cargo xtask
/// lz check`.
pub trait AbiPod: Copy + 'static {}

/// The integer primitives: initialised, any pattern valid. Not `bool` or `char`, which have
/// invalid patterns, nor pointers.
macro_rules! abi_pod_primitive {
    ($($t:ty),+) => {
        $(impl AbiPod for $t {})+
    };
}

abi_pod_primitive!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

// An array of `AbiPod` elements has no padding between them (a type's size is a multiple of
// its alignment), so it is `AbiPod` too.
impl<T: AbiPod, const N: usize> AbiPod for [T; N] {}

/// Declares the structure `$ty` [`AbiPod`] after checking, at compile time, that the fields
/// listed are its fields, in order, of exactly the types listed, each `AbiPod`, and that they
/// tile it with no padding and none left out.
///
/// ```
/// #[derive(Clone, Copy)]
/// #[repr(C)]
/// struct Timespec {
///     tv_sec: i64,
///     tv_nsec: i64,
/// }
/// hal::abi_pod!(Timespec { tv_sec: i64, tv_nsec: i64 });
/// fn main() {}
/// ```
///
/// A structure with padding is refused:
///
/// ```compile_fail
/// #[derive(Clone, Copy)]
/// #[repr(C)]
/// struct Padded {
///     a: u8,
///     b: u32,
/// }
/// hal::abi_pod!(Padded { a: u8, b: u32 });
/// fn main() {}
/// ```
///
/// So is a field whose type only dereferences to the one listed:
///
/// ```compile_fail
/// #[derive(Clone, Copy)]
/// #[repr(C)]
/// struct Ref {
///     r: &'static u64,
/// }
/// hal::abi_pod!(Ref { r: u64 });
/// fn main() {}
/// ```
///
/// So is a union (its fields overlap, so its bytes are not each a field's):
///
/// ```compile_fail
/// #[derive(Clone, Copy)]
/// #[repr(C)]
/// union Both {
///     a: u32,
///     b: u32,
/// }
/// hal::abi_pod!(Both { a: u32 });
/// fn main() {}
/// ```
///
/// And a structure with a field left out, even a zero-sized one that the tiling cannot see:
///
/// ```compile_fail
/// #[derive(Clone, Copy)]
/// #[repr(C)]
/// struct Hidden {
///     a: u64,
///     marker: core::marker::PhantomData<&'static u8>,
/// }
/// hal::abi_pod!(Hidden { a: u64 });
/// fn main() {}
/// ```
#[macro_export]
macro_rules! abi_pod {
    ($ty:path { $($field:ident : $fty:ty),+ $(,)? }) => {
        // `AbiPod`'s contract: initialised bytes, any pattern valid. The checks below prove
        // it; this macro is the only writer of the impl (`cargo xtask lz check`).
        impl $crate::user::AbiPod for $ty {}

        const _: () = {
            // Every field's type is `AbiPod`.
            const fn field_is_abi_pod<F: $crate::user::AbiPod>() {}
            $(field_is_abi_pod::<$fty>();)+
            // The fields, in the order listed, tile the structure: no padding before or
            // between them, none after, none left out.
            let mut off = 0usize;
            $(
                assert!(
                    ::core::mem::offset_of!($ty, $field) == off,
                    "abi_pod!: padding before a field, or the fields out of order"
                );
                off += ::core::mem::size_of::<$fty>();
            )+
            assert!(
                off == ::core::mem::size_of::<$ty>(),
                "abi_pod!: padding at the end, or a field left out"
            );
        };

        // Each field is exactly of the type listed: a raw pointer never deref-coerces, so a
        // field that only dereferences to it (a reference, a `Box`) does not compile here.
        // And the listed fields are all the fields of a struct: the pattern names every one
        // and has no `..`, so a field left out (a zero-sized one included) does not compile,
        // and binding a union's field by value needs `unsafe`, so a union does not either.
        // Compile time only; never called.
        const _: fn(&$ty) = |v: &$ty| {
            $(let _: *const $fty = ::core::ptr::addr_of!(v.$field);)+
            let $ty { $($field),+ } = *v;
            $(let _ = $field;)+
        };

    };
}

/// A user address of a `T` (or of the first of several): typed, never dereferenced by the
/// kernel, only copied through.
#[repr(transparent)]
pub struct UserPtr<T> {
    addr: usize,
    _t: PhantomData<fn() -> T>,
}

impl<T> Clone for UserPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for UserPtr<T> {}

impl<T> PartialEq for UserPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl<T> Eq for UserPtr<T> {}

impl<T> core::fmt::Debug for UserPtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UserPtr({:#x})", self.addr)
    }
}

impl<T> UserPtr<T> {
    /// The user address `addr`, typed (the system call's argument, as the ABI passes it).
    pub const fn new(addr: usize) -> Self {
        Self {
            addr,
            _t: PhantomData,
        }
    }

    /// The address.
    pub const fn addr(self) -> usize {
        self.addr
    }

    /// Whether the address is `NULL`.
    pub const fn is_null(self) -> bool {
        self.addr == 0
    }

    /// The address of the `n`th `T` from this one (`uaddr + n * sizeof(T)`); `None` on
    /// overflow.
    pub fn checked_add(self, n: usize) -> Option<Self> {
        let bytes = n.checked_mul(size_of::<T>())?;
        Some(Self::new(self.addr.checked_add(bytes)?))
    }
}

impl<T: AbiPod> UserPtr<T> {
    /// `copyin(uaddr, &obj, sizeof(obj))`: the `T` at this address. `EFAULT` when it is not
    /// mapped user memory.
    pub fn copyin(self) -> Result<T, Errno> {
        let mut obj = zeroed::<T>();
        self.copyin_slice(core::slice::from_mut(&mut obj))?;
        Ok(obj)
    }

    /// `copyout(&obj, uaddr, sizeof(obj))`.
    pub fn copyout(self, obj: &T) -> Result<(), Errno> {
        self.copyout_slice(core::slice::from_ref(obj))
    }

    /// `copyin(uaddr, objs, n * sizeof(T))`: `objs.len()` values from this address on.
    pub fn copyin_slice(self, objs: &mut [T]) -> Result<(), Errno> {
        let c = bridges::user_copy().ok_or(Errno::EFAULT)?;
        c.copyin(self.addr, bytes_of_mut(objs))
    }

    /// `copyout(objs, uaddr, n * sizeof(T))`.
    pub fn copyout_slice(self, objs: &[T]) -> Result<(), Errno> {
        let c = bridges::user_copy().ok_or(Errno::EFAULT)?;
        c.copyout(bytes_of(objs), self.addr)
    }
}

impl UserPtr<u8> {
    /// `copyinstr(uaddr, kbuf, kbuf.len(), &done)`: the NUL-terminated string at this address
    /// into `kbuf`; the bytes copied, the NUL included. `ENAMETOOLONG` when it does not fit,
    /// `EFAULT` when it is not mapped.
    pub fn copyinstr(self, kbuf: &mut [u8]) -> Result<usize, Errno> {
        let c = bridges::user_copy().ok_or(Errno::EFAULT)?;
        c.copyinstr(self.addr, kbuf)
    }

    /// `copyoutstr(kbuf, uaddr, kbuf.len(), &done)`: the NUL-terminated string at the start of
    /// `kbuf` to this address; the bytes copied, the NUL included.
    pub fn copyoutstr(self, kbuf: &[u8]) -> Result<usize, Errno> {
        let c = bridges::user_copy().ok_or(Errno::EFAULT)?;
        c.copyoutstr(kbuf, self.addr)
    }
}

/// The all-zero `T`.
fn zeroed<T: AbiPod>() -> T {
    // SAFETY: every bit pattern of an `AbiPod` type is a valid value (its contract, which
    // `abi_pod!` checks and nothing else implements), the all-zero one included.
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

/// The bytes of `objs`.
fn bytes_of<T: AbiPod>(objs: &[T]) -> &[u8] {
    // SAFETY: an `AbiPod` type has no padding (its contract), so every byte of the slice's
    // `size_of_val(objs)` bytes is initialised and may be read as a `u8` while `objs` is
    // borrowed; `u8` has alignment 1.
    unsafe { core::slice::from_raw_parts(objs.as_ptr().cast::<u8>(), size_of_val(objs)) }
}

/// The bytes of `objs`, writable.
fn bytes_of_mut<T: AbiPod>(objs: &mut [T]) -> &mut [u8] {
    // SAFETY: as for `bytes_of`, and since every bit pattern is a valid `T` (the contract),
    // whatever is written through the bytes leaves valid `T`s; `&mut` makes this the only
    // view while it lives.
    unsafe { core::slice::from_raw_parts_mut(objs.as_mut_ptr().cast::<u8>(), size_of_val(objs)) }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(C)]
    struct Kevent {
        ident: u64,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: i64,
        udata: u64,
    }

    crate::abi_pod!(Kevent {
        ident: u64,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: i64,
        udata: u64,
    });

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(C)]
    struct Nested {
        k: [Kevent; 2],
        tag: [u8; 8],
    }

    crate::abi_pod!(Nested {
        k: [Kevent; 2],
        tag: [u8; 8]
    });

    fn ev(i: u64) -> Kevent {
        Kevent {
            ident: i,
            filter: -1,
            flags: 0x11,
            fflags: 3,
            data: -9,
            udata: 0xdead,
        }
    }

    #[test]
    fn a_structure_round_trips_through_user_memory() {
        testutil::install();
        let u = UserPtr::<Kevent>::new(testutil::user_base());
        u.copyout(&ev(4)).unwrap();
        assert_eq!(u.copyin(), Ok(ev(4)));
    }

    #[test]
    fn slices_and_nested_structures_round_trip() {
        testutil::install();
        let u = UserPtr::<Nested>::new(testutil::user_base() + 1024);
        let n = Nested {
            k: [ev(1), ev(2)],
            tag: *b"kqueue\0\0",
        };
        u.copyout_slice(&[n, n]).unwrap();
        let mut back = [zeroed::<Nested>(); 2];
        u.copyin_slice(&mut back).unwrap();
        assert_eq!(back, [n, n]);
        assert_eq!(
            u.checked_add(1).map(UserPtr::addr),
            Some(u.addr() + size_of::<Nested>())
        );
    }

    #[test]
    fn an_unmapped_address_is_efault() {
        testutil::install();
        assert_eq!(UserPtr::<u32>::new(8).copyin(), Err(Errno::EFAULT));
        assert_eq!(
            UserPtr::<u32>::new(usize::MAX - 2).copyout(&1),
            Err(Errno::EFAULT)
        );
        assert_eq!(UserPtr::<u64>::new(usize::MAX).checked_add(1), None);
    }

    #[test]
    fn a_string_is_copied_with_its_nul_or_refused() {
        testutil::install();
        let u = UserPtr::<u8>::new(testutil::user_base() + 2048);
        assert_eq!(u.copyoutstr(b"/etc/rc\0"), Ok(8));
        let mut k = [0u8; 16];
        assert_eq!(u.copyinstr(&mut k), Ok(8));
        assert_eq!(&k[..8], b"/etc/rc\0");
        let mut short = [0u8; 4];
        assert_eq!(u.copyinstr(&mut short), Err(Errno::ENAMETOOLONG));
    }
}
/* </TESTS> */
