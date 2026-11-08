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
//! `sys/hal/trust.rs`: the address of a socket address (`struct sockaddr *`) that legacy
//! storage holds and lends whole, `ifa_addr`, `ifa_dstaddr`, `ifa_netmask` and `ifma_addr` of
//! an interface address `bsd`'s internet protocols own (`struct in_ifaddr`'s `ia_addr`,
//! `in6_ifaddr`'s, `in_multi`'s `inm_sin`). The kernel's interface layer compares and copies
//! those addresses (`ifa_ifwithaddr`, `ifaof_ifpforaddr`, `ifconf`), so it reads their
//! `sa_len` bytes through [`SaLink::view`].
//!
//! ## Redesign
//! - `Cell<*const T>` with "valid while linked" -> `Link<T>` over `'static` targets: the
//!   validity is the type's, so it is checked by the compiler at `set`, not promised at every
//!   read. `Sync` by composition (an atomic pointer and a `PhantomData<&'static T>`), so a
//!   link may be shared exactly when `T` may.
//! - N6.3a: LZ's `Cell<*mut Sockaddr>` members of `struct ifaddr` and `struct ifmaddr`, read
//!   through an `unsafe` deref at each of `if.c`'s readers ("the address lives while the
//!   address is linked") -> [`SaLink`]: the word `bsd` sets, and one view of its `sa_len`
//!   bytes whose premise is written once, in `sys/hal/trust.rs`; it goes when `in_ifaddr` and
//!   `in6_ifaddr` are the kernel's and the interface address embeds its storage (N6.4, N6.5).

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::sys::socket::Sockaddr;

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
/// `struct sockaddr *` of an interface address whose storage legacy code owns and lends
/// whole (`ifa_addr`, `ifa_dstaddr`, `ifa_netmask`, `ifma_addr`): the word `bsd` sets
/// ([`SaLink::set`], LZ's `Cell::set` of the member) and reads back ([`SaLink::get`], for its
/// own derefs), and the kernel's view of the address's `sa_len` bytes ([`SaLink::view`]).
///
/// The view rests on the premise `sys/hal/trust.rs` registers (the "socket address lent by
/// an interface address" paragraph): a word that is not null is the address of a socket
/// address whose storage holds at least its two header bytes and its `sa_len` bytes, made
/// with that storage's provenance (`Cell::as_ptr` of the member that holds it), which lives
/// while the interface address that holds the word lives, and whose bytes are written only
/// by a holder of the net lock in its exclusive mode, while the kernel reads them only under
/// the net lock (the C's `[N]`). Nothing here can check it; it goes when the storage is the
/// kernel's (N6.4, N6.5).
#[repr(transparent)]
pub struct SaLink(AtomicPtr<Sockaddr>);

impl SaLink {
    /// `NULL`.
    pub const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    /// Stores `sa` (`ifa->ifa_addr = sintosa(&ia->ia_addr)`): null, or a socket address
    /// whose storage keeps the premise of the type's docs.
    #[inline]
    pub fn set(&self, sa: *const Sockaddr) {
        self.0.store(sa.cast_mut(), Ordering::Release);
    }

    /// The address, for a legacy reader that dereferences it in its own `unsafe` block, as
    /// LZ's sites do (relaxed: the word changes under the net lock its readers hold).
    #[inline]
    pub fn get(&self) -> *mut Sockaddr {
        self.0.load(Ordering::Relaxed)
    }

    /// Whether the word is null.
    #[inline]
    pub fn is_none(&self) -> bool {
        self.0.load(Ordering::Relaxed).is_null()
    }

    /// The socket address's `sa_len` bytes (its header's first byte), borrowed for as long as
    /// the link is: `None` when the word is null. For a holder of the net lock (the C's
    /// readers of `ifa_addr`).
    #[inline]
    pub fn view(&self) -> Option<&[u8]> {
        let p = self.0.load(Ordering::Acquire).cast::<u8>().cast_const();
        if p.is_null() {
            return None;
        }
        // SAFETY: by the premise this type documents and `sys/hal/trust.rs` registers, a word
        // that is not null is the address, with its storage's provenance, of a socket address
        // whose storage holds at least its two header bytes and its `sa_len` bytes: reading
        // the first byte (`sa_len`), then a slice of `sa_len` bytes from the start, stays
        // inside one allocation of initialised bytes, at an alignment of 1. The storage lives
        // while the interface address holding this link does, and this link is borrowed for
        // the slice's life, so the bytes outlive the slice. They are written only by a holder
        // of the net lock in its exclusive mode and the caller holds the net lock, so no write
        // happens while the slice lives, and the shared slice aliases no `&mut`. The `Acquire`
        // load pairs with `set`'s `Release`, so the bytes written before the word was
        // published are seen.
        Some(unsafe { core::slice::from_raw_parts(p, usize::from(p.read())) })
    }
}

impl Default for SaLink {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for SaLink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SaLink({:p})", self.0.load(Ordering::Relaxed))
    }
}
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
        // SAFETY: a `SaLink` is an `AtomicPtr`: zero is null.
        let s: SaLink = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        assert!(s.is_none() && s.view().is_none());
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

    #[test]
    fn a_sockaddr_is_viewed_as_its_sa_len_bytes() {
        // An in_ifaddr's ia_addr (16 bytes) and an in6_ifaddr's (28): the storage lends its
        // own provenance, as bsd's `Cell::as_ptr` does.
        let sin = Box::leak(Box::new(core::cell::Cell::new([0u8; 16])));
        let mut b = [0u8; 16];
        b[0] = 16;
        b[1] = 2;
        b[4..8].copy_from_slice(&[10, 0, 2, 15]);
        sin.set(b);
        let sin6 = Box::leak(Box::new(core::cell::Cell::new([0u8; 28])));
        let mut b6 = [0u8; 28];
        b6[0] = 28;
        b6[1] = 24;
        b6[27] = 0xee;
        sin6.set(b6);

        let l = SaLink::new();
        assert!(l.view().is_none());
        l.set(sin.as_ptr().cast());
        assert_eq!(l.view(), Some(&b[..]));
        assert!(ptr::eq(l.get().cast_const(), sin.as_ptr().cast()));
        l.set(sin6.as_ptr().cast());
        let v = l.view().unwrap();
        assert_eq!((v.len(), v[1], v[27]), (28, 24, 0xee));
        // A shorter sa_len views only its bytes (a netmask's).
        let mut short = [0u8; 16];
        short[0] = 5;
        sin.set(short);
        l.set(sin.as_ptr().cast());
        assert_eq!(l.view().map(<[u8]>::len), Some(5));
        l.set(ptr::null());
        assert!(l.is_none());
        fn shared<T: Send + Sync>() {}
        shared::<SaLink>();
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
