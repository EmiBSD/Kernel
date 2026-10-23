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
//! [`UserPtr::copyinstr`] a NUL-terminated string. The copies themselves are the machine's,
//! `hal`'s own since N4.8b (`arch/arm64/copy.rs`: `copy.S` and `copystr.S` with
//! `pcb_onfault`; the host's double, `arch/host/copy.rs`): safe functions whose kernel side
//! is the byte slice they are given and whose user side the assembly checks (user half, no
//! overflow, unprivileged accesses, a fault `EFAULT`), so they alias nothing of the kernel's.
//! The machine's kernel copies (`kcopy_in`, `kcopy_out`) reach kernel memory by address and
//! are `unsafe`: `hal` calls them only through a [`ForeignWindow`] and the one
//! [`KcopyToken`], below.
//!
//! A [`ForeignWindow`] is a run of kernel virtual addresses onto pages of another address
//! space (`uvm_io`'s `uvm_map_extract` of a process's range into `kernel_map` for
//! `kern.proc_args`, `sys_kbind`'s of the caller's own page): those pages are also mapped
//! and written by their own process, so no Rust reference to them may exist, and a page may
//! go away under the copy. The window is never referenced: [`ForeignWindow::read_at`] and
//! [`ForeignWindow::write_at`] copy through it with the machine's `kcopy`, which turns a
//! fault into `EFAULT` (`pcb_onfault`), as the C's `uiomove`/`kcopy` on the window do. Only
//! the kernel map's token mints one ([`KernelMapper::foreign_window`]): its holder vouches,
//! as for every address it maps, that the range is its own and that no Rust object lives in
//! it (`sys/hal/trust.rs`); the window bounds every copy (one past its end panics).
//!
//! A [`KcopyToken`] is the one holder of `kcopy` onto kernel addresses it names itself:
//! `bsd`'s `uiomove`, whose `UIO_SYSSPACE` iovecs are kernel buffers their builders vouch
//! for (`struct uio`'s contract), takes it once ([`KcopyToken::take`]). It is decision 30's
//! documented trust, as the memory bridges' (`sys/hal/trust.rs`): nothing here can check
//! the addresses, and the token goes with `UioMove` when the last `UIO_SYSSPACE` builder is
//! typed (N7).
//!
//! **Where `hal` has no copy of the machine's** (amd64 until N10, the ABI module's runtime,
//! which copies through `KernelApi`): every copy fails with `EFAULT`, as before a
//! registration did.
//!
//! ## Redesign
//! - `copyin(uaddr, &obj, sizeof(obj))` -> `UserPtr::<T>::new(uaddr).copyin()`, returning the
//!   value; `copyout(&obj, uaddr, sizeof(obj))` -> `.copyout(&obj)`. The size is the type's,
//!   so it cannot disagree with the object.
//! - The `unsafe impl AbiPod for T {}` of each owner -> `abi_pod!(T { field: Type, ... })`
//!   beside the type, checked by the compiler; the legacy trait of `sys/machine/copy.rs` stays
//!   for its legacy users until each type's owner moves.
//! - N3.5 S0: `kcopy(src, dst, len)` onto a window of extracted pages, an `unsafe fn` over two
//!   raw pointers in LZ (`uvm_mmap.rs` `sys_kbind`, `uvm_io.rs`'s slice over the window) ->
//!   [`ForeignWindow`], minted by the kernel map's token, whose `read_at`/`write_at` copy a
//!   bounded range through the bridge's `kcopy_in`/`kcopy_out`: the copy is the machine's,
//!   with its fault recovery; the kernel never holds a reference to the foreign pages.
//! - N4.7a: an ioctl's argument arrives in the kernel as the bytes `sys_ioctl` copied in
//!   (`FileOps::ioctl`'s `data: &mut [u8]`, the C's `caddr_t data` cast to the command's
//!   structure); [`pod_from_bytes`] and [`pod_to_bytes`] are that cast for an `AbiPod` type,
//!   a copy over the byte views below (no `unsafe` of their own), bounded by the slice. LZ's
//!   `ioctl_arg`/`ioctl_ret` (`sys/sys/ioctl.rs`, three `unsafe` over the legacy trait) stay
//!   for their legacy users.
//! - N4.6b: [`bytes_of`] and [`bytes_of_mut`] are public (no new `unsafe`: the two blocks were
//!   here), so that `bsd`'s `SysctlPlain` is a blanket over `AbiPod` and its 12 `unsafe impl`s
//!   go.
//! - N4.8b: the `UserCopy` bridge is gone: the copies are the machine's, `hal`'s own
//!   (`crate::arch::current::copy`); `bsd`'s `uiomove`'s `UIO_SYSSPACE` leg, an `unsafe`
//!   `kcopy` over raw pointers in LZ (`kern_subr.rs`), is the [`KcopyToken`]'s safe copy
//!   (the user's decision 13 of N4.8: `kcopy` itself `pub(crate)`, behind the window and the
//!   token).

use core::marker::PhantomData;
use core::mem::MaybeUninit;

use core::sync::atomic::{AtomicBool, Ordering};

use crate::assert::hal_panic;
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
use crate::pmap::KernelMapper;
use crate::sys::errno::Errno;
use crate::sys::types::Vaddr;

/// The machine's copies (`copy.S`, `copystr.S`; the host's double).
#[cfg(any(
    not(target_os = "none"),
    all(target_arch = "aarch64", not(feature = "module-rt"))
))]
use crate::arch::current::copy as machine;

/// No copy of the machine's in `hal` (amd64 until N10, the ABI module's runtime): every copy
/// is `EFAULT`, as before the `UserCopy` bridge was registered.
#[cfg(not(any(
    not(target_os = "none"),
    all(target_arch = "aarch64", not(feature = "module-rt"))
)))]
mod machine {
    use crate::sys::errno::Errno;

    /// `copyin`: no user address space.
    pub(super) fn copyin(_uaddr: usize, _kbuf: &mut [u8]) -> Result<(), Errno> {
        Err(Errno::EFAULT)
    }
    /// `copyout`.
    pub(super) fn copyout(_kbuf: &[u8], _uaddr: usize) -> Result<(), Errno> {
        Err(Errno::EFAULT)
    }
    /// `copyinstr`.
    pub(super) fn copyinstr(_uaddr: usize, _kbuf: &mut [u8]) -> Result<usize, Errno> {
        Err(Errno::EFAULT)
    }
    /// `copyoutstr`.
    pub(super) fn copyoutstr(_kbuf: &[u8], _uaddr: usize) -> Result<usize, Errno> {
        Err(Errno::EFAULT)
    }
    /// `copyin32`.
    pub(super) fn copyin32(_uaddr: usize) -> Result<u32, Errno> {
        Err(Errno::EFAULT)
    }
}

/// A run of kernel virtual addresses onto pages of another address space (`uvm_io`'s and
/// `sys_kbind`'s extracted window): copied through, never referenced. Made only by the
/// kernel map's token ([`KernelMapper::foreign_window`]), which it borrows.
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
pub struct ForeignWindow<'m> {
    va: Vaddr,
    len: usize,
    _token: &'m KernelMapper,
}

#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
impl ForeignWindow<'_> {
    /// The window's first address.
    pub fn va(&self) -> Vaddr {
        self.va
    }

    /// The window's length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `kcopy(va + off, buf, buf.len())`: the window's bytes from `off` into `buf`; `EFAULT`
    /// when a page faults (or where `hal` has no copy). A range past the window's end panics:
    /// the C would copy beyond what it extracted.
    pub fn read_at(&self, off: usize, buf: &mut [u8]) -> Result<(), Errno> {
        let kva = self.at(off, buf.len());
        if buf.is_empty() {
            return Ok(());
        }
        #[cfg(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        ))]
        // SAFETY: `[kva, kva + buf.len())` lies inside the window (`at` checked it), which the
        // kernel map's token minted over a range of foreign pages its own `uvm_map_extract`
        // put there, holding no Rust object, and keeps mapped while the window lives (its
        // contract, `foreign_window`); the pages are also written by their own process, which
        // the copy tolerates (it is the C's `kcopy`, no reference made), and a page that went
        // away is `EFAULT` (`pcb_onfault`). `buf` is the caller's, borrowed for the copy.
        let r = unsafe { machine::kcopy_in(kva, buf) };
        #[cfg(not(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        )))]
        let r = {
            let _ = kva;
            Err(Errno::EFAULT)
        };
        r
    }

    /// `kcopy(src, va + off, src.len())`: `src` into the window from `off`; `EFAULT` when a
    /// page faults (or where `hal` has no copy). A range past the window's end panics.
    pub fn write_at(&self, off: usize, src: &[u8]) -> Result<(), Errno> {
        let kva = self.at(off, src.len());
        if src.is_empty() {
            return Ok(());
        }
        #[cfg(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        ))]
        // SAFETY: as in `read_at`: the range lies inside the window, foreign pages no Rust
        // object occupies, mapped while the window lives; a page that went away is `EFAULT`.
        let r = unsafe { machine::kcopy_out(src, kva) };
        #[cfg(not(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        )))]
        let r = {
            let _ = kva;
            Err(Errno::EFAULT)
        };
        r
    }

    /// The address of `n` bytes at `off`, checked to lie inside the window.
    fn at(&self, off: usize, n: usize) -> usize {
        match off.checked_add(n) {
            Some(end) if end <= self.len => self.va.as_usize() + off,
            _ => hal_panic(format_args!(
                "ForeignWindow: {n} bytes at {off:#x} past the window's {:#x}",
                self.len
            )),
        }
    }
}

#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
impl KernelMapper {
    /// The window `[va, va + len)` onto foreign pages the kernel map holds there (`uvm_io`'s
    /// and `sys_kbind`'s extracted range). Nothing here checks the range: `write_at` through
    /// a window is a write to any kernel address its maker names, the strongest primitive the
    /// token gives. The token's contract, beside every address it maps: its holder mints a
    /// window only over a range its own `uvm_map_extract` returned into the kernel map
    /// (pages of another address space, no Rust object among them), and unmaps that range
    /// only after the window is gone. (N3.5 S1 makes that a type: windows minted only from
    /// the extract's result.) A range that wraps the address space panics.
    pub fn foreign_window(&self, va: Vaddr, len: usize) -> ForeignWindow<'_> {
        if va.as_usize().checked_add(len).is_none() {
            hal_panic(format_args!(
                "foreign_window: {len:#x} bytes at {:#x} wrap the address space",
                va.as_usize()
            ));
        }
        ForeignWindow {
            va,
            len,
            _token: self,
        }
    }
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
    #[inline]
    pub fn copyin_slice(self, objs: &mut [T]) -> Result<(), Errno> {
        machine::copyin(self.addr, bytes_of_mut(objs))
    }

    /// `copyout(objs, uaddr, n * sizeof(T))`.
    #[inline]
    pub fn copyout_slice(self, objs: &[T]) -> Result<(), Errno> {
        machine::copyout(bytes_of(objs), self.addr)
    }
}

impl UserPtr<u8> {
    /// `copyinstr(uaddr, kbuf, kbuf.len(), &done)`: the NUL-terminated string at this address
    /// into `kbuf`; the bytes copied, the NUL included. `ENAMETOOLONG` when it does not fit,
    /// `EFAULT` when it is not mapped.
    pub fn copyinstr(self, kbuf: &mut [u8]) -> Result<usize, Errno> {
        machine::copyinstr(self.addr, kbuf)
    }

    /// `copyoutstr(kbuf, uaddr, kbuf.len(), &done)`: the NUL-terminated string at the start of
    /// `kbuf` to this address; the bytes copied, the NUL included.
    pub fn copyoutstr(self, kbuf: &[u8]) -> Result<usize, Errno> {
        machine::copyoutstr(kbuf, self.addr)
    }
}

impl UserPtr<u32> {
    /// `copyin32(uaddr, &v)`: the 32-bit word at this address in one access, as a futex is
    /// read; `EFAULT` when it is misaligned or not mapped.
    pub fn copyin32(self) -> Result<u32, Errno> {
        machine::copyin32(self.addr)
    }
}

/// The right to `kcopy` onto kernel addresses its holder names itself (`bsd`'s `uiomove`'s
/// `UIO_SYSSPACE` leg, the user's decision 13 of N4.8): one per kernel, taken once, by
/// `bsd`'s boot before any code of the kernel or ABI crates runs (`hal_bridges_register`'s
/// first step), so none of them can hold it: their later takes get `None`.
///
/// The holder's contract (decision 30's documented trust, `sys/hal/trust.rs`): every address
/// it passes is a kernel buffer of at least the length copied that its builder owns for the
/// copy (a `UIO_SYSSPACE` iovec's, `struct uio`'s contract), holding no Rust object that
/// something else references during the copy; nothing here can check it. A page that is
/// not mapped is `EFAULT` (`pcb_onfault`), as the C's `kcopy`. Gone with `UioMove` at N7.
pub struct KcopyToken {
    _private: (),
}

/// The one token.
static KCOPY_TOKEN: KcopyToken = KcopyToken { _private: () };
/// Whether [`KCOPY_TOKEN`] was taken.
static KCOPY_TAKEN: AtomicBool = AtomicBool::new(false);

impl KcopyToken {
    /// The token, to its first taker only; `None` afterwards.
    pub fn take() -> Option<&'static KcopyToken> {
        if KCOPY_TAKEN.swap(true, Ordering::AcqRel) {
            None
        } else {
            Some(&KCOPY_TOKEN)
        }
    }

    /// `kcopy(kva, buf, buf.len())`: the bytes at the kernel address `kva` into `buf`;
    /// `EFAULT` when a page faults (or where `hal` has no copy).
    pub fn kcopy_in(&self, kva: usize, buf: &mut [u8]) -> Result<(), Errno> {
        #[cfg(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        ))]
        // SAFETY: the token's contract (the type's docs): `kva` names a kernel buffer of at
        // least `buf.len()` bytes its holder vouches for, no Rust object written meanwhile; a
        // page that is not mapped is `EFAULT`. `buf` is the caller's, borrowed for the copy.
        let r = unsafe { machine::kcopy_in(kva, buf) };
        #[cfg(not(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        )))]
        let r = {
            let _ = (kva, buf);
            Err(Errno::EFAULT)
        };
        r
    }

    /// `kcopy(buf, kva, buf.len())`: `buf` to the kernel address `kva`; `EFAULT` when a page
    /// faults (or where `hal` has no copy).
    pub fn kcopy_out(&self, buf: &[u8], kva: usize) -> Result<(), Errno> {
        #[cfg(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        ))]
        // SAFETY: the token's contract: `kva` names a kernel buffer of at least `buf.len()`
        // bytes its holder vouches for, which no Rust reference covers during the copy.
        let r = unsafe { machine::kcopy_out(buf, kva) };
        #[cfg(not(any(
            not(target_os = "none"),
            all(target_arch = "aarch64", not(feature = "module-rt"))
        )))]
        let r = {
            let _ = (buf, kva);
            Err(Errno::EFAULT)
        };
        r
    }
}

/// The `T` an ioctl's argument bytes hold (`*(struct winsize *)data`): its first
/// `size_of::<T>()` bytes, copied out; `None` when `b` is shorter (the C's `IOCPARM_LEN`
/// always covers it, so a short slice is a caller's mistake the kernel reports, never reads
/// past).
pub fn pod_from_bytes<T: AbiPod>(b: &[u8]) -> Option<T> {
    let src = b.get(..size_of::<T>())?;
    let mut v = zeroed::<T>();
    bytes_of_mut(core::slice::from_mut(&mut v)).copy_from_slice(src);
    Some(v)
}

/// `*(struct winsize *)data = *v`: the bytes of `v` into the first `size_of::<T>()` bytes of
/// an ioctl's argument; `false` (nothing written) when `b` is shorter.
pub fn pod_to_bytes<T: AbiPod>(v: &T, b: &mut [u8]) -> bool {
    match b.get_mut(..size_of::<T>()) {
        Some(dst) => {
            dst.copy_from_slice(bytes_of(core::slice::from_ref(v)));
            true
        }
        None => false,
    }
}

/// The all-zero `T`.
pub fn zeroed<T: AbiPod>() -> T {
    // SAFETY: every bit pattern of an `AbiPod` type is a valid value (its contract, which
    // `abi_pod!` checks and nothing else implements), the all-zero one included.
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

/// The bytes of `objs`.
pub fn bytes_of<T: AbiPod>(objs: &[T]) -> &[u8] {
    // SAFETY: an `AbiPod` type has no padding (its contract), so every byte of the slice's
    // `size_of_val(objs)` bytes is initialised and may be read as a `u8` while `objs` is
    // borrowed; `u8` has alignment 1.
    unsafe { core::slice::from_raw_parts(objs.as_ptr().cast::<u8>(), size_of_val(objs)) }
}

/// The bytes of `objs`, writable.
pub fn bytes_of_mut<T: AbiPod>(objs: &mut [T]) -> &mut [u8] {
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
    fn an_ioctl_argument_reads_and_writes_as_its_structure() {
        // The bytes of a `struct kevent` as sys_ioctl hands them over (a little longer than
        // the structure: IOCPARM_LEN rounds nothing here, the slice may be longer).
        let mut data = [0u8; 40];
        assert!(pod_to_bytes(&ev(7), &mut data));
        assert_eq!(&data[..8], &7u64.to_ne_bytes());
        assert_eq!(&data[8..10], &(-1i16).to_ne_bytes());
        assert_eq!(
            &data[32..],
            &[0u8; 8],
            "past the structure nothing is written"
        );
        assert_eq!(pod_from_bytes::<Kevent>(&data), Some(ev(7)));
        assert_eq!(
            pod_from_bytes::<i32>(&[1, 0, 0, 0, 9]),
            Some(i32::from_ne_bytes([1, 0, 0, 0]))
        );
        // A slice shorter than the structure: refused, nothing read or written.
        assert_eq!(pod_from_bytes::<Kevent>(&data[..31]), None);
        let mut short = [0xeeu8; 31];
        assert!(!pod_to_bytes(&ev(1), &mut short));
        assert_eq!(short, [0xee; 31]);
        // A nested structure round-trips too.
        let n = Nested {
            k: [ev(1), ev(2)],
            tag: *b"abcdefgh",
        };
        let mut nb = [0u8; size_of::<Nested>()];
        assert!(pod_to_bytes(&n, &mut nb));
        assert_eq!(pod_from_bytes::<Nested>(&nb), Some(n));
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
    fn a_word_is_read_aligned_or_refused() {
        testutil::install();
        let at = testutil::user_base() + 3072;
        UserPtr::<u32>::new(at).copyout(&0x1234_5678).unwrap();
        assert_eq!(UserPtr::<u32>::new(at).copyin32(), Ok(0x1234_5678));
        // copyin32's alignment check: a misaligned word is EFAULT even when mapped.
        assert_eq!(UserPtr::<u32>::new(at + 2).copyin32(), Err(Errno::EFAULT));
        assert_eq!(UserPtr::<u32>::new(8).copyin32(), Err(Errno::EFAULT));
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

    #[test]
    fn a_foreign_window_copies_through_the_bridge() {
        testutil::install();
        let base = testutil::foreign_base();
        let w = testutil::pmap_boot()
            .mapper
            .foreign_window(Vaddr::new(base + 64), 128);
        assert_eq!(
            (w.va(), w.len(), w.is_empty()),
            (Vaddr::new(base + 64), 128, false)
        );
        w.write_at(8, b"kbind").unwrap();
        let mut back = [0u8; 5];
        w.read_at(8, &mut back).unwrap();
        assert_eq!(&back, b"kbind");
        assert_eq!(testutil::foreign_bytes(64 + 8, 5), b"kbind");
        // The last byte, and an empty copy at the end.
        w.write_at(127, &[9]).unwrap();
        w.read_at(128, &mut []).unwrap();
        assert_eq!(testutil::foreign_bytes(64 + 127, 1), [9]);
    }

    #[test]
    fn a_foreign_window_onto_nothing_faults() {
        testutil::install();
        let base = testutil::foreign_base();
        let w = testutil::pmap_boot()
            .mapper
            .foreign_window(Vaddr::new(base + testutil::FOREIGN_LEN - 4), 16);
        let mut buf = [0u8; 16];
        assert_eq!(w.read_at(0, &mut buf), Err(Errno::EFAULT));
        assert_eq!(w.write_at(0, &buf), Err(Errno::EFAULT));
        assert_eq!(w.read_at(0, &mut buf[..4]), Ok(()));
    }

    #[test]
    #[should_panic(expected = "past the window's")]
    fn a_copy_past_the_window_panics() {
        testutil::install();
        let w = testutil::pmap_boot()
            .mapper
            .foreign_window(Vaddr::new(testutil::foreign_base()), 16);
        let mut buf = [0u8; 4];
        let _ = w.read_at(13, &mut buf);
    }

    #[test]
    #[should_panic(expected = "past the window's")]
    fn an_offset_that_overflows_panics() {
        testutil::install();
        let w = testutil::pmap_boot()
            .mapper
            .foreign_window(Vaddr::new(testutil::foreign_base()), 16);
        let _ = w.write_at(usize::MAX, &[1]);
    }

    #[test]
    #[should_panic(expected = "wrap the address space")]
    fn a_window_that_wraps_panics() {
        testutil::install();
        let _ = testutil::pmap_boot()
            .mapper
            .foreign_window(Vaddr::new(usize::MAX - 2), 16);
    }

    /// The token goes to its first taker alone, and copies kernel memory both ways: here the
    /// host's own buffers (the token's holder vouches for them), a fault in the doubles'
    /// address space being `EFAULT`.
    #[test]
    fn the_kcopy_token_is_taken_once_and_copies() {
        let t = KcopyToken::take().expect("the first take");
        assert!(KcopyToken::take().is_none(), "a second take");
        let src = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut dst = [0u8; 8];
        t.kcopy_in(core::ptr::from_ref(&src).expose_provenance(), &mut dst)
            .unwrap();
        assert_eq!(dst, src);
        let mut out = [0u8; 8];
        t.kcopy_out(&src[..4], core::ptr::from_mut(&mut out).expose_provenance())
            .unwrap();
        assert_eq!(out, [1, 2, 3, 4, 0, 0, 0, 0]);
        let mut buf = [0u8; 4];
        assert_eq!(t.kcopy_in(0x10, &mut buf), Err(Errno::EFAULT));
        assert_eq!(t.kcopy_out(&buf, 0x10), Err(Errno::EFAULT));
    }
}
/* </TESTS> */
