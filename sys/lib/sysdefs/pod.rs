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
//! The `AbiPod` marker and the `abi_pod!` macro that implements it (decision 54): a data
//! shape, so it lives with the types it marks. The byte reinterpretations that trust the
//! marker (`hal::user::bytes_of`, `pod_from_bytes`, `UserPtr`'s copies, `hal::mem::pod`) are
//! `hal`'s: this crate holds no `unsafe` and no logic.
//!
//! ## Redesign
//! - `sysdefs::pod::AbiPod` and `sysdefs::abi_pod!` (N3.5, decision 17) moved here unchanged, with the
//!   doc tests of the macro; the macro's expansion names `$crate::pod::AbiPod`.

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
/// sysdefs::abi_pod!(Timespec { tv_sec: i64, tv_nsec: i64 });
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
/// sysdefs::abi_pod!(Padded { a: u8, b: u32 });
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
/// sysdefs::abi_pod!(Ref { r: u64 });
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
/// sysdefs::abi_pod!(Both { a: u32 });
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
/// sysdefs::abi_pod!(Hidden { a: u64 });
/// fn main() {}
/// ```
#[macro_export]
macro_rules! abi_pod {
    ($ty:path { $($field:ident : $fty:ty),+ $(,)? }) => {
        // `AbiPod`'s contract: initialised bytes, any pattern valid. The checks below prove
        // it; this macro is the only writer of the impl (`cargo xtask lz check`).
        impl $crate::pod::AbiPod for $ty {}

        const _: () = {
            // Every field's type is `AbiPod`.
            const fn field_is_abi_pod<F: $crate::pod::AbiPod>() {}
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
        #[allow(
            clippy::used_underscore_binding,
            reason = "the check binds every field by its name, the named holes too"
        )]
        const _: fn(&$ty) = |v: &$ty| {
            $(let _: *const $fty = ::core::ptr::addr_of!(v.$field);)+
            let $ty { $($field),+ } = *v;
            $(let _ = $field;)+
        };

    };
}
/* </CODE> */
