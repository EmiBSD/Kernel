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
//! `Link<T>`: a settable reference to something that lives for ever.
//!
//! Not an OpenBSD file. The C links the objects that are never freed (the `vm_page` array and
//! the pmemranges stolen at boot, `uvm_page_init`) with plain pointers, `NULL` for none; LZ
//! spells them `Cell<*const T>` or `Cell<Option<NonNull<T>>>`, each read in its own `unsafe`
//! block whose comment says the target is still there. A `Link<T>` holds only a `&'static T`
//! (or nothing): [`Link::set`] takes the reference, so whatever it names outlives every
//! reader, and [`Link::get`] gives it back. That is the whole contract, and the one `unsafe`
//! of the type, in `get`, rests on it alone: no lock and no caller's promise is involved, so
//! the links of [`crate::list::List`] and [`crate::tree::Tree`], which are `Link`s, have safe
//! mutators.
//!
//! The pointer is an atomic (`Release` on store, `Acquire` on load), so a value built and then
//! published through a link is seen whole by a CPU that follows it. Which lock keeps a set of
//! links consistent (a list's, a tree's) is the owner's business: a race between two writers
//! can make a list wrong, never a reference dangle.
//!
//! [`SyncLink<T>`] (N5.3) is a `Link` for a member whose type would make the compiler prove
//! `T: Sync` through a cycle of explicit `Send`/`Sync` impls: a descriptor table's directories
//! (`fd_cdir`, `fd_rdir`) and a session's terminal (`s_ttyvp`) are vnodes, and a vnode's
//! knotes lead through a kqueue back to its descriptor table, whose counted handle
//! (`PoolArc`) is `Sync` only when the table is; that cycle of `T: Send + Sync` bounds is
//! inductive and the compiler gives up on it (`E0275`). A `SyncLink` is `Send` and `Sync`
//! whatever `T` is, and lends its target only when `T: Sync`, which is what sharing it needs.
//!
//! [`PtrWord<T>`] (N4.2) is the other half of LZ's `Cell<*const T>`: an address that the type
//! never follows, for a member whose target is not `'static` and is still dereferenced by
//! legacy code under its own `unsafe` (`ps_fd`, `ps_sigacts`, `p_kq` while those subsystems
//! are `bsd`'s): a word, so the struct that holds it may be `Send` and `Sync`.
//!
//! [`SaLink`] (N6.3a) is a `PtrWord` the kernel does follow, under a premise registered in
//! `sys/hal/trust.rs`: the address of a socket address (`struct sockaddr *`) whose storage
//! `bsd`'s internet protocols own, `ifa_addr`, `ifa_dstaddr`, `ifa_netmask` and `ifma_addr`
//! of an interface address (`struct in_ifaddr`'s `ia_addr`, `in6_ifaddr`'s, `in_multi`'s
//! `inm_sin`). The kernel's interface layer compares and copies those addresses
//! (`ifa_ifwithaddr`, `ifaof_ifpforaddr`, `ifconf`), so it reads their `sa_len` bytes
//! through [`SaLink::read`]: a copy, made under the net lock (a `NetHeld` proves it), of a
//! storage that
//! only the holder of the one [`SaToken`] (`bsd`'s boot takes it) can publish, typed
//! (`&'static Cell<T>` of an `AbiPod` image) so the copy never passes its end.
//!
//! ## Redesign
//! - `Cell<*const T>` with "valid while linked" -> `Link<T>` over `'static` targets: the
//!   validity is the type's, so it is checked by the compiler at `set`, not promised at every
//!   read. `Sync` by composition (an atomic pointer and a `PhantomData<&'static T>`), so a
//!   link may be shared exactly when `T` may.
//! - N6.3a: LZ's `Cell<*mut Sockaddr>` members of `struct ifaddr` and `struct ifmaddr`, read
//!   through an `unsafe` deref at each of `if.c`'s readers ("the address lives while the
//!   address is linked") -> [`SaLink`]: the storage `bsd` publishes with the [`SaToken`] as a
//!   typed `&'static Cell<T>` (its size kept beside the address, the pair changed and read
//!   under a sequence so a reader never pairs one storage with another's size), and one
//!   copy of its bytes, cut at that size, under a net lock the copy checks; the premise left
//!   (the token holder writes the bytes only under the exclusive net lock; a reader under a
//!   shared holding is one of the readers) is written once, in `sys/hal/trust.rs`. It goes
//!   when `in_ifaddr` and `in6_ifaddr` are the kernel's and the interface address embeds its
//!   storage (N6.4, N6.5). Review 1's B2: the first shape (`set(*const Sockaddr)` from safe
//!   code, a `'static` slice lent of whatever `sa_len` said, no lock checked) let a safe
//!   caller read past any storage or keep a slice across `ifafree`.

use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicU32, Ordering, fence};

use crate::assert::hal_panic;
use crate::netlock::NetHeld;
use crate::rwlock::{RW_READ, RW_WRITE};
use crate::sys::socket::Sockaddr;
use crate::user::AbiPod;

/// The longest socket address an [`SaLink`] reads (`struct sockaddr_dl`'s 32 bytes; the
/// internet ones are 16 and 28): the size of [`SaBytes`]'s buffer, and the largest image
/// [`SaLink::set`] accepts.
pub const SA_READ_MAX: usize = 32;

/// A reference to a `T` that lives for ever, or none (`NULL`), settable through `&self`.
///
/// Invariant: `ptr` is null or the address of a `&'static T` given to [`Link::set`]; nothing
/// else ever writes it.
pub struct Link<T: 'static> {
    ptr: AtomicPtr<T>,
    /// The link stands for an `Option<&'static T>`: `Send` and `Sync` exactly when that is.
    _ref: PhantomData<&'static T>,
}

impl<T: 'static> Link<T> {
    /// No target (`NULL`).
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            _ref: PhantomData,
        }
    }

    /// A link to `to` from the start.
    pub const fn to(to: &'static T) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::from_ref(to).cast_mut()),
            _ref: PhantomData,
        }
    }

    /// The target, `None` when the link is empty.
    #[inline]
    pub fn get(&self) -> Option<&'static T> {
        let p = self.ptr.load(Ordering::Acquire);
        if p.is_null() {
            return None;
        }
        // SAFETY: by the type's invariant `p` is the address of a `&'static T` that `set` (or
        // `to`) was given, the only writers: it is aligned, points to a `T` valid for the
        // whole run of the kernel, and the shared reference made here is exactly the one
        // that was handed in, so it aliases only shared references. The `Acquire` load pairs
        // with the `Release` store of `set`, so the `T` as it was built before it was linked
        // is visible here.
        Some(unsafe { &*p })
    }

    /// Points the link at `to` (`None` empties it).
    #[inline]
    pub fn set(&self, to: Option<&'static T>) {
        let p = to.map_or(ptr::null_mut(), |r| ptr::from_ref(r).cast_mut());
        self.ptr.store(p, Ordering::Release);
    }

    /// Whether the link is empty (`== NULL`).
    #[inline]
    pub fn is_none(&self) -> bool {
        self.ptr.load(Ordering::Relaxed).is_null()
    }

    /// Whether the link names `x` (an address comparison, nothing followed).
    #[inline]
    pub fn points_to(&self, x: &T) -> bool {
        ptr::eq(self.ptr.load(Ordering::Relaxed), ptr::from_ref(x))
    }

    /// The target's address as a number, 0 when the link is empty: what a reader that only
    /// prints or compares the address takes (`kern.file`'s `v_data` of a socket's vnode,
    /// `kern_sysctl.c`), without making a reference to a target another CPU may be changing.
    #[inline]
    pub fn addr(&self) -> usize {
        self.ptr.load(Ordering::Relaxed).addr()
    }
}

impl<T: 'static> Default for Link<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> core::fmt::Debug for Link<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Link({:p})", self.ptr.load(Ordering::Relaxed))
    }
}

/// A reference to a `T` that lives for ever, or none (`NULL`), settable through `&self`, as
/// [`Link`], but `Send` and `Sync` whatever `T` is (the module's notes): its target is lent
/// and stored only for a `T: Sync`, so no thread can reach a `T` through it that `T` does not
/// allow to be shared.
///
/// Invariant: `ptr` is null or the address of a `&'static T` given to [`SyncLink::set`],
/// [`SyncLink::replace`] (its swap) or [`SyncLink::to`] for a `T: Sync`; nothing else ever
/// writes it.
pub struct SyncLink<T: 'static> {
    ptr: AtomicPtr<T>,
}

impl<T: 'static> SyncLink<T> {
    /// No target (`NULL`).
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Whether the link is empty (`== NULL`).
    #[inline]
    pub fn is_none(&self) -> bool {
        self.ptr.load(Ordering::Relaxed).is_null()
    }

    /// The target's address as a number, 0 when the link is empty (nothing followed).
    #[inline]
    pub fn addr(&self) -> usize {
        self.ptr.load(Ordering::Relaxed).addr()
    }

    /// Whether the link names `x` (an address comparison, nothing followed).
    #[inline]
    pub fn points_to(&self, x: &T) -> bool {
        ptr::eq(self.ptr.load(Ordering::Relaxed), ptr::from_ref(x))
    }
}

impl<T: Sync + 'static> SyncLink<T> {
    /// A link to `to` from the start.
    pub const fn to(to: &'static T) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::from_ref(to).cast_mut()),
        }
    }

    /// The target, `None` when the link is empty.
    #[inline]
    pub fn get(&self) -> Option<&'static T> {
        self.read(Access::Load)
    }

    /// Points the link at `to` (`None` empties it).
    #[inline]
    pub fn set(&self, to: Option<&'static T>) {
        self.ptr.store(Self::word(to), Ordering::Release);
    }

    /// Points the link at `to`, returning the target it had: one atomic swap (`xchg`).
    #[inline]
    pub fn replace(&self, to: Option<&'static T>) -> Option<&'static T> {
        self.read(Access::Swap(to))
    }

    /// The pointer word of `to` (null for `None`).
    #[inline]
    fn word(to: Option<&'static T>) -> *mut T {
        to.map_or(ptr::null_mut(), |r| ptr::from_ref(r).cast_mut())
    }

    /// The target the link has (`Access::Load`), or had before it was swapped to another
    /// (`Access::Swap`: one atomic exchange), as a reference.
    #[inline]
    fn read(&self, access: Access<T>) -> Option<&'static T> {
        let p = match access {
            Access::Load => self.ptr.load(Ordering::Acquire),
            Access::Swap(to) => self.ptr.swap(Self::word(to), Ordering::AcqRel),
        };
        if p.is_null() {
            return None;
        }
        // SAFETY: `p` is a value of `self.ptr` (the load's, or the swap's previous one), and
        // by the type's invariant every value it holds is null or the address of a
        // `&'static T` that `set`, `replace` or `to` was given, the only writers, each from a
        // reference: it is aligned, points to a `T` valid for the whole run of the kernel,
        // and the shared reference made here is exactly the one that was handed in, so it
        // aliases only shared references. `T: Sync` (this impl's bound), so handing that
        // reference to whichever thread shares the link is sound, although the link's own
        // `Send`/`Sync` did not ask it. The `Acquire` half of the load or the swap pairs
        // with the `Release` half of the store or swap that wrote `p`, so the `T` as it was
        // built before it was linked is visible here.
        Some(unsafe { &*p })
    }
}

/// What [`SyncLink`]'s reader does to the word: read it, or exchange it for a new target.
enum Access<T: 'static> {
    /// A load.
    Load,
    /// A swap to this target (`None` empties the link).
    Swap(Option<&'static T>),
}

// A reference is `Copy` whatever `T` is, so the choice is too (a derive would ask `T: Copy`).
impl<T: 'static> Clone for Access<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Access<T> {}

impl<T: 'static> Default for SyncLink<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> core::fmt::Debug for SyncLink<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SyncLink({:p})", self.ptr.load(Ordering::Relaxed))
    }
}

/// An address kept as a word (`struct filedesc *ps_fd` while the descriptors are `bsd`'s): LZ's
/// `Cell<*const T>` with its `get()`/`set()`, made `Send` and `Sync` by being an atomic, not by
/// a premise. Nothing here follows the pointer: whoever dereferences what `get` returns does
/// it in its own `unsafe` block with its own argument, as LZ's sites do today.
#[repr(transparent)]
pub struct PtrWord<T>(AtomicPtr<T>);

impl<T> PtrWord<T> {
    /// `NULL`.
    pub const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    /// The address (relaxed: the word is written and read under the lock its owner names).
    #[inline]
    pub fn get(&self) -> *const T {
        self.0.load(Ordering::Relaxed).cast_const()
    }

    /// The address, for a member the C declares non-`const` (`ps_uvpaths`).
    #[inline]
    pub fn get_mut(&self) -> *mut T {
        self.0.load(Ordering::Relaxed)
    }

    /// Stores `p`.
    #[inline]
    pub fn set(&self, p: *const T) {
        self.0.store(p.cast_mut(), Ordering::Relaxed);
    }

    /// Stores `p`, returning the address it replaces.
    #[inline]
    pub fn replace(&self, p: *const T) -> *const T {
        self.0.swap(p.cast_mut(), Ordering::Relaxed).cast_const()
    }
}

impl<T> Default for PtrWord<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> core::fmt::Debug for PtrWord<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PtrWord({:p})", self.0.load(Ordering::Relaxed))
    }
}

/// The capability to make an [`SaLink`] name a socket address's storage ([`SaLink::set`]):
/// one per kernel, taken by `bsd`'s boot before any code of the kernel or ABI crates runs
/// (`hal_bridges_register`'s first steps), for the internet protocols that own the storage of
/// the interface addresses until N6.4 and N6.5. Its holder's contract is the "socket address
/// lent by an interface address" paragraph of `sys/hal/trust.rs`.
pub struct SaToken {
    /// Not constructible outside this module.
    _private: (),
}

impl SaToken {
    /// The token, to its first taker only; `None` afterwards.
    pub fn take() -> Option<&'static SaToken> {
        if SA_TOKEN_TAKEN.swap(true, Ordering::AcqRel) {
            None
        } else {
            Some(&SA_TOKEN)
        }
    }

    /// The proof that the net lock is held, for the token holder's walks under a holding of
    /// its own that no witness exists of (`bsd`'s detached `NET_LOCK()` and
    /// `NET_LOCK_SHARED()`): `None` unless the lock is written by the running process or
    /// read (the C's `NET_ASSERT_LOCKED()`). That a read holding is the caller's, `rw_status`
    /// cannot say: the holder's contract (`sys/hal/trust.rs`), which also asks it to drop the
    /// proof before it lets the holding go (the proof of a `&'static` token is
    /// `NetHeld<'static>`: no borrow ends it). Kernel code holds a witness and takes
    /// [`NetHeld::of`] instead.
    pub fn net_held(&self) -> Option<NetHeld<'_>> {
        let held = crate::netlock::status();
        (held == RW_WRITE || held == RW_READ).then(NetHeld::new)
    }
}

/// A copy of the socket address an [`SaLink`] names, read under the net lock: its `sa_len`
/// bytes, cut at the size of the storage, and its two header bytes whatever `sa_len` says
/// (the C reads `sa_family` of every address it walks).
#[derive(Clone, Copy)]
pub struct SaBytes {
    /// How many bytes of `buf` are the address (`min(sa_len, storage)`).
    len: u8,
    /// The bytes read; at least the two of the header.
    buf: [u8; SA_READ_MAX],
}

impl SaBytes {
    /// `sa_family`.
    #[inline]
    pub fn family(&self) -> u8 {
        self.buf[1]
    }

    /// The address's bytes: `sa_len` of them, or as many as its storage holds.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }
}

impl core::ops::Deref for SaBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl core::fmt::Debug for SaBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SaBytes({:?})", self.as_bytes())
    }
}

/// `struct sockaddr *` of an interface address (`ifa_addr`, `ifa_dstaddr`, `ifa_netmask`,
/// `ifma_addr`) whose storage the internet protocols own until N6.4 and N6.5 (`struct
/// in_ifaddr`'s `ia_addr`, ...): the storage's address, which `bsd` publishes ([`SaLink::set`],
/// with the [`SaToken`], LZ's `Cell::set` of the member) and reads back for its own derefs
/// ([`SaLink::get`]), and a copy of its bytes for the kernel's interface layer
/// ([`SaLink::read`], under the net lock, which it checks).
///
/// Invariant: `ptr` is null, or `set` stored it with `cap` = `size_of::<T>()` from a
/// `&'static Cell<T>` (`T: AbiPod`, at least 2 and at most [`SA_READ_MAX`] bytes, by `set`'s
/// compile-time check), or `set_word` stored it with `cap` 0 (a word never read); the two
/// change only together, inside `seq`'s odd window, so a reader that sees `seq` even and
/// unchanged around its two loads has a pair one writer stored. All zero is a valid, empty
/// link (null, 0, 0): an interface address in a zeroed softc or `M_ZERO` allocation.
pub struct SaLink {
    /// The storage's address with its provenance (`Cell::as_ptr`), null for none.
    ptr: AtomicPtr<u8>,
    /// The storage's size; 0 for a word [`SaLink::set_word`] stored.
    cap: AtomicU8,
    /// The writers' sequence: odd while one changes `ptr` and `cap`.
    seq: AtomicU32,
}

impl SaLink {
    /// `NULL`.
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            cap: AtomicU8::new(0),
            seq: AtomicU32::new(0),
        }
    }

    /// `ifa->ifa_addr = sintosa(&ia->ia_addr)`: the link names `sa`'s storage (`None` empties
    /// it), whose bytes [`SaLink::read`] copies. For the [`SaToken`]'s holder, under its
    /// contract (the type's docs).
    #[inline]
    pub fn set<T: AbiPod>(&self, _token: &SaToken, sa: Option<&'static Cell<T>>) {
        const {
            assert!(
                size_of::<T>() >= 2 && size_of::<T>() <= SA_READ_MAX,
                "SaLink::set: not a socket address the link can read"
            );
        }
        let p = sa.map_or(ptr::null_mut(), |c| c.as_ptr().cast::<u8>());
        // `size_of::<T>()` fits: at most SA_READ_MAX, checked above.
        self.publish(p, size_of::<T>() as u8);
    }

    /// The link empties (`ifa->ifa_dstaddr = NULL`). Needs no token: an empty link lends
    /// nothing.
    #[inline]
    pub fn clear(&self) {
        self.publish(ptr::null_mut(), 0);
    }

    /// A socket address's word that is never read through the link (`enc(4)`'s
    /// link-level address on its private `ifaddr`, which no interface lists): `bsd`'s own
    /// derefs of [`SaLink::get`] alone follow it; [`SaLink::read`] of it is `panic(9)`. For
    /// the [`SaToken`]'s holder: `bsd` dereferences what `get` gives, so no other code may
    /// plant a word (review 2's nit; the `PtrWord` precedent). N6.3b: a kernel `rtsock` that
    /// reads `rt_ifa->ifa_addr` of enc's routes would reach this word and stop; enc's address
    /// must be published as a storage (or the reader take `get`) by then.
    #[inline]
    pub fn set_word(&self, _token: &SaToken, sa: *const Sockaddr) {
        self.publish(sa.cast::<u8>().cast_mut(), 0);
    }

    /// Stores the pair inside the writers' window: the sequence made odd by one writer at a
    /// time (the C's writers hold the net lock exclusively, so the wait is never long), the
    /// two stores, the sequence even again.
    fn publish(&self, p: *mut u8, cap: u8) {
        let mut s = self.seq.load(Ordering::Relaxed);
        loop {
            if s & 1 == 0 {
                match self.seq.compare_exchange_weak(
                    s,
                    s.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(now) => s = now,
                }
            } else {
                core::hint::spin_loop();
                s = self.seq.load(Ordering::Relaxed);
            }
        }
        // A reader whose loads see either store sees the odd sequence after them.
        fence(Ordering::Release);
        self.ptr.store(p, Ordering::Relaxed);
        self.cap.store(cap, Ordering::Relaxed);
        // The storage's bytes, written before, are seen by a reader that sees this.
        self.seq.store(s.wrapping_add(2), Ordering::Release);
    }

    /// The pair one writer stored (the type's invariant).
    #[inline]
    fn load_pair(&self) -> (*const u8, u8) {
        loop {
            let s = self.seq.load(Ordering::Acquire);
            if s & 1 == 0 {
                let p = self.ptr.load(Ordering::Relaxed);
                let cap = self.cap.load(Ordering::Relaxed);
                fence(Ordering::Acquire);
                if self.seq.load(Ordering::Relaxed) == s {
                    return (p.cast_const(), cap);
                }
            }
            core::hint::spin_loop();
        }
    }

    /// The address, for a legacy reader that dereferences it in its own `unsafe` block, as
    /// LZ's sites do (relaxed: the word changes under the net lock its readers hold).
    #[inline]
    pub fn get(&self) -> *mut Sockaddr {
        self.ptr.load(Ordering::Relaxed).cast::<Sockaddr>()
    }

    /// Whether the word is null.
    #[inline]
    pub fn is_none(&self) -> bool {
        self.ptr.load(Ordering::Relaxed).is_null()
    }

    /// A copy of the socket address (`None` when the link is empty), for a holder of the net
    /// lock (the C's readers of `ifa_addr`, `NET_ASSERT_LOCKED()`), which `_held` proves (a
    /// witness's, or the token holder's check, made once per walk): `panic(9)` when the link
    /// holds a word [`SaLink::set_word`] stored.
    pub fn read(&self, _held: &NetHeld<'_>) -> Option<SaBytes> {
        let (p, cap) = self.load_pair();
        if p.is_null() {
            return None;
        }
        if cap == 0 {
            hal_panic(format_args!(
                "SaLink: a word that is never read was read ({p:p})"
            ));
        }
        let cap = usize::from(cap).min(SA_READ_MAX);
        let mut out = SaBytes {
            len: 0,
            buf: [0; SA_READ_MAX],
        };
        // SAFETY: the invariant (`load_pair`'s pair, one writer's): `p` is the address, with
        // its provenance, of the storage of a `&'static Cell<T>` that `set` was given, `T:
        // AbiPod` and `cap = size_of::<T>()` between 2 and SA_READ_MAX (`set`'s compile-time
        // check; `set_word`'s words have `cap` 0 and were refused above). That storage is
        // `cap` initialised bytes (an `AbiPod` has no padding) that `Cell::as_ptr` lets be read
        // through a shared cell, it lives for ever (`'static`: a later `set` repointing the
        // link leaves it alive), and its alignment is at least 1: reading byte 0, then
        // `n = max(2, min(sa_len, cap)) <= cap` bytes into `out.buf` (SA_READ_MAX long, at
        // least `cap`) stays inside it and inside the buffer, which is a distinct local. No
        // write races with the copy: only the `SaToken`'s holder can make a link name a cell
        // (taken by `bsd`'s boot before any `forbid` crate runs), and its contract
        // (`sys/hal/trust.rs`) is that the cell's bytes are written only while it holds the
        // net lock exclusively, and `_held` proves the caller holds the lock while it copies:
        // a witness of it (`NetHeld::of`: kernel code, exact, so no exclusive holder exists
        // until the caller lets go), or the token holder's own check of a holding of its own
        // (`SaToken::net_held`, under the holder's contract that a read holding it finds is
        // its own). The copy makes no reference to the storage, so a write after it touches
        // nothing the caller holds.
        out.len = unsafe {
            let len = usize::from(p.read()).min(cap);
            ptr::copy_nonoverlapping(p, out.buf.as_mut_ptr(), len.max(2));
            len as u8
        };
        Some(out)
    }
}

impl Default for SaLink {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for SaLink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SaLink({:p})", self.ptr.load(Ordering::Relaxed))
    }
}

/// The one [`SaToken`].
static SA_TOKEN: SaToken = SaToken { _private: () };

/// Whether [`SA_TOKEN`] was taken.
static SA_TOKEN_TAKEN: AtomicBool = AtomicBool::new(false);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::thread;

    use super::*;

    static A: u32 = 7;
    static B: u32 = 9;

    #[test]
    fn empty_then_set_then_emptied() {
        let l = Link::<u32>::new();
        assert!(l.is_none());
        assert_eq!(l.get(), None);
        l.set(Some(&A));
        assert_eq!(l.get(), Some(&7));
        assert!(l.points_to(&A));
        assert!(!l.points_to(&B));
        l.set(Some(&B));
        assert_eq!(l.get().copied(), Some(9));
        l.set(None);
        assert!(l.is_none());
        assert_eq!(Link::to(&A).get(), Some(&7));
    }

    #[test]
    fn a_sync_link_is_shared_whatever_its_target_and_lends_a_sync_one() {
        fn shared<T: Send + Sync>() {}
        // A target that is not Sync: the link is shareable all the same (it lends nothing).
        shared::<SyncLink<core::cell::Cell<u8>>>();
        let l = SyncLink::<u32>::new();
        assert!(l.is_none() && l.get().is_none() && l.addr() == 0);
        l.set(Some(&A));
        assert_eq!(l.get(), Some(&7));
        assert!(l.points_to(&A) && !l.points_to(&B));
        assert_eq!(l.addr(), ptr::from_ref(&A).addr());
        assert_eq!(l.replace(Some(&B)), Some(&7));
        assert_eq!(l.get().copied(), Some(9));
        l.set(None);
        assert!(l.is_none());
        assert_eq!(SyncLink::to(&A).get(), Some(&7));
    }

    /// `replace` is one exchange: threads swapping their own targets in see every target
    /// that was in the link exactly once (a load then a store would hand one out twice and
    /// lose another).
    #[test]
    fn replace_is_one_exchange() {
        const THREADS: usize = 4;
        const SWAPS: usize = 2000;
        let values: &'static [u32] =
            Box::leak((0..=(THREADS * SWAPS) as u32).collect::<Box<[u32]>>());
        let l: &'static SyncLink<u32> = Box::leak(Box::new(SyncLink::to(&values[0])));
        let workers: std::vec::Vec<_> = (0..THREADS)
            .map(|t| {
                thread::spawn(move || {
                    let mut olds = std::vec::Vec::new();
                    for i in 0..SWAPS {
                        let v = &values[1 + t * SWAPS + i];
                        olds.push(*l.replace(Some(v)).unwrap());
                    }
                    olds
                })
            })
            .collect();
        let mut seen: std::vec::Vec<u32> = workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect();
        seen.push(*l.get().unwrap());
        seen.sort_unstable();
        assert_eq!(seen, values);
    }

    #[test]
    fn the_address_is_the_targets_or_zero() {
        let l = Link::<u32>::new();
        assert_eq!(l.addr(), 0);
        l.set(Some(&B));
        assert_eq!(l.addr(), ptr::from_ref(&B).addr());
        l.set(None);
        assert_eq!(l.addr(), 0);
    }

    #[test]
    fn an_address_word_is_only_kept() {
        let w = PtrWord::<u32>::new();
        assert!(w.get().is_null());
        w.set(&raw const A);
        assert!(ptr::eq(w.get(), &raw const A));
        assert!(ptr::eq(w.replace(&raw const B), &raw const A));
        assert!(ptr::eq(w.get_mut().cast_const(), &raw const B));
        fn shared<T: Send + Sync>() {}
        shared::<PtrWord<core::cell::Cell<u8>>>();
    }

    #[test]
    fn an_all_zero_link_and_word_are_null() {
        // A zeroed object that embeds links (an interface in an `M_ZERO` softc) holds empty
        // ones.
        // SAFETY: a `Link` is an `AtomicPtr` (zero is null, which `get` answers `None`) and a
        // `PhantomData`: all zero bytes are a valid, empty link.
        let l: Link<u32> = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        assert!(l.is_none() && l.get().is_none());
        l.set(Some(&A));
        assert_eq!(l.get(), Some(&7));
        // SAFETY: a `PtrWord` is an `AtomicPtr`: zero is null.
        let w: PtrWord<u32> = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        assert!(w.get().is_null());
        // SAFETY: a `SaLink` is an `AtomicPtr` (zero is null), an `AtomicU8` and an
        // `AtomicU32`: all zero bytes are an empty link whose sequence is even.
        let s: SaLink = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        assert!(
            s.is_none() && s.read(&crate::netlock::NetHeld::new()).is_none() && s.get().is_null()
        );
    }

    /// The embed idiom's proof (docs/IDIOMS.md) for `crate::list` (a `forbid(unsafe_code)`
    /// module, so its zeroed values are built here): a head and an entry in a zeroed legacy
    /// object (an interface's address list in an `M_ZERO` softc) are an empty list and an
    /// unlinked entry.
    #[test]
    fn an_all_zero_list_is_empty_and_an_all_zero_entry_unlinked() {
        use crate::list::{List, ListEntry};

        struct Item {
            q: ListEntry<Item>,
        }
        crate::list_adapter!(Q: Item, q);

        // SAFETY: a `List` is two `Link`s (`AtomicPtr`s: zero is null) and a `PhantomData`; a
        // `ListEntry` two `Link`s and an `AtomicBool` (zero is `false`): all zero bytes are an
        // empty head and an entry on no list.
        let l: List<Q> = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        assert!(l.is_empty() && l.first().is_none() && l.last().is_none());
        // SAFETY: as above.
        let q: ListEntry<Item> = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        let a: &'static Item = Box::leak(Box::new(Item { q }));
        assert!(!List::<Q>::is_linked(a));
        l.insert_tail(a);
        assert!(List::<Q>::is_linked(a) && l.first().is_some());
        l.remove(a);
        assert!(l.is_empty() && !List::<Q>::is_linked(a));
    }

    /// The one token, for every test of this binary (taken once).
    fn token() -> &'static SaToken {
        static T: std::sync::OnceLock<&'static SaToken> = std::sync::OnceLock::new();
        T.get_or_init(|| SaToken::take().expect("the first take"))
    }

    /// A leaked socket address image of `N` bytes: `sa_len`, the family, then `rest`.
    fn image<const N: usize>(sa_len: u8, family: u8, rest: &[u8]) -> &'static Cell<[u8; N]> {
        let mut b = [0u8; N];
        b[0] = sa_len;
        b[1] = family;
        b[2..2 + rest.len()].copy_from_slice(rest);
        Box::leak(Box::new(Cell::new(b)))
    }

    #[test]
    fn a_socket_address_is_read_as_its_sa_len_bytes_cut_at_its_storage() {
        crate::testutil::install();
        let _s = crate::netlock::tests::serial();
        let net = crate::netlock::NETLOCK.write();
        let held = crate::netlock::NetHeld::of(&net).expect("the net lock's witness");

        // An in_ifaddr's ia_addr (16 bytes) and an in6_ifaddr's (28).
        let sin = image::<16>(16, 2, &[0, 0, 10, 0, 2, 15]);
        let sin6 = image::<28>(28, 24, &[0; 26]);
        let l = SaLink::new();
        assert!(l.read(&held).is_none());
        l.set(token(), Some(sin));
        let r = l.read(&held).expect("an address");
        assert_eq!((r.family(), r.as_bytes()), (2, &sin.get()[..]));
        assert!(ptr::eq(l.get().cast_const().cast(), sin.as_ptr()));
        l.set(token(), Some(sin6));
        let r = l.read(&held).expect("an address");
        assert_eq!((r.len(), r[1]), (28, 24));

        // A shorter sa_len reads only its bytes (a netmask's), the header whatever it says.
        let mask = image::<16>(5, 2, &[0, 0, 255]);
        l.set(token(), Some(mask));
        assert_eq!(l.read(&held).map(|r| (r.len(), r.family())), Some((5, 2)));
        let zero = image::<16>(0, 2, &[]);
        l.set(token(), Some(zero));
        assert_eq!(l.read(&held).map(|r| (r.len(), r.family())), Some((0, 2)));
        // A longer sa_len than its storage stops at the storage (review 1's B2).
        let long = image::<16>(200, 2, &[1; 14]);
        l.set(token(), Some(long));
        let r = l.read(&held).expect("an address");
        assert_eq!(r.as_bytes(), &long.get()[..]);

        // The copy is the caller's: the storage written afterwards changes nothing in it.
        long.set([9; 16]);
        assert_eq!(
            r.as_bytes(),
            &[200, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1][..]
        );

        l.clear();
        assert!(l.is_none() && l.read(&held).is_none());
        fn shared<T: Send + Sync>() {}
        shared::<SaLink>();
    }

    #[test]
    fn a_read_needs_a_proof_of_the_net_lock_and_a_word_is_never_read() {
        crate::testutil::install();
        let _s = crate::netlock::tests::serial();
        let l: &'static SaLink = Box::leak(Box::new(SaLink::new()));
        l.set(token(), Some(image::<16>(16, 2, &[])));
        // Neither written by this thread nor read: the token holder gets no proof.
        assert!(token().net_held().is_none());
        // Read: the witness's proof, and the token holder's check.
        let r = crate::netlock::NETLOCK.read();
        let held = crate::netlock::NetHeld::of(&r).expect("the net lock's witness");
        assert_eq!(l.read(&held).map(|r| r.len()), Some(16));
        assert!(token().net_held().is_some());
        drop(r);
        // Another lock's witness proves nothing.
        static OTHER: crate::rwlock::RawRwlock = crate::rwlock::RawRwlock::new("other");
        let o = OTHER.read();
        assert!(crate::netlock::NetHeld::of(&o).is_none());
        drop(o);
        let w = crate::netlock::NETLOCK.write();
        let held = crate::netlock::NetHeld::of(&w).expect("the net lock's witness");
        assert_eq!(l.read(&held).map(|r| r.len()), Some(16));
        // Written by another thread: the token holder gets no proof there.
        thread::scope(|s| {
            s.spawn(|| assert!(token().net_held().is_none()));
        });
        // enc(4)'s word: kept for bsd, never read.
        let sa = image::<16>(16, 18, &[]);
        l.set_word(token(), sa.as_ptr().cast());
        assert!(ptr::eq(l.get().cast_const().cast(), sa.as_ptr()));
        assert!(std::panic::catch_unwind(|| l.read(&crate::netlock::NetHeld::new())).is_err());
        drop(held);
        drop(w);
    }

    /// A reader never pairs one storage with the other's size: two writers switch the link
    /// between a 16-byte and a 32-byte storage, both of whose `sa_len` say 32, while two
    /// readers read; a torn pair would read 32 bytes of the 16-byte one.
    #[test]
    fn a_read_sees_one_writers_pair() {
        crate::testutil::install();
        let _s = crate::netlock::tests::serial();
        let l: &'static SaLink = Box::leak(Box::new(SaLink::new()));
        l.set(token(), Some(image::<16>(32, 16, &[16; 14])));
        thread::scope(|s| {
            for _ in 0..2 {
                s.spawn(|| {
                    // A cell is not `Sync`: each writer publishes storages of its own.
                    let short = image::<16>(32, 16, &[16; 14]);
                    let long = image::<32>(32, 32, &[32; 30]);
                    for i in 0..20_000 {
                        if i % 2 == 0 {
                            l.set(token(), Some(long));
                        } else {
                            l.set(token(), Some(short));
                        }
                    }
                });
            }
            for _ in 0..2 {
                s.spawn(|| {
                    let r = crate::netlock::NETLOCK.read();
                    let held = crate::netlock::NetHeld::of(&r).expect("the net lock's witness");
                    for _ in 0..20_000 {
                        let b = l.read(&held).expect("an address");
                        assert_eq!(usize::from(b.family()), b.len());
                        assert!(b.iter().skip(2).all(|&x| x == b.family()));
                    }
                });
            }
        });
    }

    #[test]
    fn the_token_is_taken_once() {
        let _ = token();
        assert!(SaToken::take().is_none());
    }

    #[test]
    fn a_value_published_on_one_cpu_is_seen_whole_on_another() {
        static L: Link<[u64; 4]> = Link::new();
        let v: &'static [u64; 4] = Box::leak(Box::new([1, 2, 3, 4]));
        let t = thread::spawn(move || L.set(Some(v)));
        t.join().unwrap();
        assert_eq!(L.get(), Some(&[1, 2, 3, 4]));
    }
}
/* </TESTS> */
