---
name: redesigner-design
description: Rust and ownership idioms of the redesign: witnesses, guards, intrusive lists, cells, pools
metadata:
  type: feedback
---

- Intrusive links: derive the entry pointer with `NonNull::from(elem).cast::<u8>().map_addr(|a| a.saturating_add(OFFSET))`, never from `&elem.field`, so container_of stays sound (docs/IDIOMS.md typed-handle row).
- Safe readers over intrusive links are sound only if unlink clears the element's links and a poison value is flagged, never followed; tree(3) defines element ops only for linked elements, so clearing is no deviation.
- `crate::crypto::wipe(x)` is `*x = T::default()`, which drops the old value: a `Drop` calling `wipe(self)` recurses; wipe field by field in a `zeroize` (`explicit_bzero` arrays over 32) called by `Drop` and `finalize`.
- A non-Copy key in a `Cell` has no safe by-reference read and `take()`/`set()` exposes the zero key to other CPUs: a shared run-time key is `AtomicSiphashKey` (IDIOMS row); one in a StaticCell is read with `get()`.
- `#[cfg(..)] { value }` as a tail expression is unstable: write `if cfg!(feature = "..") { a } else { b }`.
- `abi_pod!`-style tiling checks (`offset_of!` + sizes) miss unions and omitted zero-sized fields; an exhaustive `let $T { $($f),+ } = *v;` in a never-called closure refuses both.
- OpenBSD opens BOTH pipe files `FREAD | FWRITE` (sys_pipe.c dopipe): which pipe a file holds cannot come from `f_flag`; carry it with the handle.
- `klist_assert_locked` with `kl_ops == None` asserts KERNEL_LOCK under diagnostic + MP: an MPSAFE object cannot use an ops-less klist with the `_locked` forms.
- A `Native` syscall entry must still run bsd's prologue (mi_syscall: refreshcreds, pin_check, pledge_syscall, the kernel-lock rule) on its own sysent row with the syscall number, or the module path silently drops pledge and pinsyscalls.
- bsd's `Klist`/`SigioRef` (anything with `SlistHead`/raw-pointer Cells) are Sync but not Send, so they cannot sit in a `PoolArc` value; the glue holds a leaked, recycled `&'static` (pipe_glue's PipeKlists, IDIOMS row).
- A kernel macro and a fn of the same name in one module clash at `pub(crate) use` (E0255): name the helper differently (`report_unported`).
- A setter taking `&WriteWitness` cannot run while a `Guarded` cell of that lock is open through `get_mut(&mut w)`: counters under a foreign lock are relaxed atomics whose setters `kassert!` the lock.
- A function that makes a value and returns a witness borrowing it takes a `&'v mut Option<T>` slot: `let v: &'v T = slot.insert(t);` returns `(&'v T, Witness<'v>)`.
- Machine lock: release only what you took; take-run-release in a script that releases only after its own `mkdir` succeeded (N3.4c once removed another agent's lock).
- A kernel trait method implemented by bsd over functions taking `&'static Vnode` can declare `fn m(&'static self)` (object-safe for `&'static dyn`).
- An owning container's `remove(&mut self, e: &T)` cannot take an `e` borrowed from it (E0502): give find-and-remove by key (`OwnedTree::take_by`) and validate elements by a container identity in their entry.
- `Guarded::get_mut(&mut w)` borrows the witness for the reference's life: two `&mut` cells of one lock need the witness split into parts (`Divisible::split`); parts borrow the witness, so return a value holding them (`MapSplit`) with `parts(&mut self)` (IDIOMS row). Check a brief's guard signatures against this before freezing an API.
- bsd's `uiomove` over one `UIO_SYSSPACE` iovec is a safe path to `kcopy` (pcb_onfault); making `kcopy` safe would launder legacy blocks and give bsd a safe arbitrary-address write.
- Over an owned intrusive tree, name elements by unique key across inserts, removals and clips (clip_start returns the clip address); a loop that clips its own cursor updates `first` when `key == first`.
- A trait method reading lock-guarded fields without a witness parameter cannot be implemented over `Guarded`: move such reads to the guard trait (`g.aref(e)`).
- `std::panic::catch_unwind(|| f(&x))` stops compiling once `x` gains cells: wrap in `AssertUnwindSafe`.
- Coding against an API another agent writes in parallel: keep its assumed signatures as a never-committed stub file and a script that swaps it out before each commit and back after.
- A mechanical `use` rewrite that splits one `use` into several drops the `#[cfg]` on all but the first: list HEAD's attributed uses and fix those by hand.
- Derived state cached inside `MapState` (Owners, ~180 B) grows `Vmspace` past `the_pool_items_stay_near_the_cs`'s 512 B (vmsppl item size, vmstat -m): compute once per operation and lend `&T` down instead (as fast, no size change).
- A `&'s VmMapEntry` returned from a `&'s mut MapState` keeps the whole state borrowed; inside a loop that reborrows the state NLL refuses it (universal region): return a `Copy` summary, or reborrow with `OwnedTree::insert_get` + destructuring `let MapState { addr, uaddr_exe, .. } = st` for split borrows.
- LZ frees pool items and softcs without dropping them (`proc_free` is a raw `pool_put`; tcpcb, syn cache, sessions likewise): an owning handle (`PoolArc`, `MBox`) inside a legacy struct leaks there; grep the struct's free path before giving it an owning field.
- A task's `pending` kept as its pinned entry's link word (one atomic, cleared by the unlink itself) avoids touching an element after unlinking it, which a separate flag cleared after the unlink would need.
- `PoolArc::from_ref(&T)` needs proof the value is in a pool item (a `PoolHome` field recorded by `get_homed`, `pool_home!`): a bare `&T` may be a `static` (proc0, P4) or a stack value.
- Before planning a type move, check every frozen accessor against its field's lock class: `[o]` handle fields (`p.ucred() -> &Ucred`, `p.vmspace()`) cannot keep a `&` API in a kernel type without a per-thread token (N4.5's `current()`) or an embedded atomic copy.
- zsh: an unquoted `grep --include=*.rs` fails "no matches found" before grep runs; quote the glob (or put the loop in a script file).
- A macro arm `$(impl[..])? $ty:ty` is ambiguous (`impl Trait` is a type): give the generic form its own arm first, keyed on the literal `impl`.
- A kernel type's inherent method shadows a bsd extension trait's method of the same name (Thread::vmspace vs ProcExt::vmspace): keep kernel names off bsd's frozen accessors.
- Once a bsd type becomes an alias of a kernel type, bsd's `impl Proc {}` blocks stop compiling (E0116): move them to the glue's member type, which auto-deref reaches.
- s_count is held by ttys (TIOCSCTTY's sesshold) and tprintf_open too, not only by groups: a counted SessionRef needs holders there.
- An API that removes an element given `&T` cannot be fed a `&T` borrowed from the same guard (E0502): take the element's key (`p_seq`), which the owner reads from itself.
- `if c && let Some(x) = slot.lock().take() { body }` keeps the guard alive through `body` (if-let temporaries): take into a `let` first when `body` may sleep (frele, a close).
- A handle that must take its PoolArc out in `Drop` (`FileRef(Option<PoolArc>)`) loses the null niche: `Option<Handle>` becomes two words and grows every pool item holding one; give nullable holders a one-word slot over the handle's own empty state (kernel::file::FileSlot).
- A per-element Drop on a 20-slot inline table cost ~1 ns a slot in release (fdcopy-20 +37 %); an is_none fast path and a #[cold] close did not move it: measure the table's make+drop row before choosing a Drop.
- A Bash command ending in a stdin reader (`cat > /dev/null`) hangs until the 2-minute timeout and goes to the background: stop it with TaskStop; never leave a stdin consumer in a command.
- A C switch table as a trait (decision 46, N4.4b `Filter<G>`): static methods plus a `const fn of<F: Trait>() -> Table` filling `Some(F::method)` compiles on stable (fn items coerce to fn pointers in const fn), an inline `const { assert!(F::FLAGS ..) }` inside refuses bad flags at compile time, and `&Kqueue::<G>::CONST` of the result is still promoted to `&'static`.
- When an erased table's `None` meant something to the dispatch (the C's NULL: `knote_modify` at splhigh when not MPSAFE, a fault when MPSAFE), make the trait's default method reproduce it by reading `Self::FLAGS`, so `of` can fill every entry and no behaviour moves.
- `&Type::<G>::CONST` is promoted afresh at each site: two promotions of one associated const can have different addresses, so test an erased table (`Filterops`) by its flags and by calling its entries, never `ptr::eq` (N4.7a LogRead).
- A kernel object whose state needs the glue (a `Klist<G>`, a `FileRef<G::FileGlue>`) cannot be a generic `static`: put it in `ProcTables<G>` (`ProcTables::log`); a timeout there is `Timeout::new(f::<G>, null)` in the const fn and `pin_static(&G::tables().x.tick)`; in tests a wheel run on another thread sees that thread's (empty) tables, so its callback must tolerate them.
