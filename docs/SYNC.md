# Syncing with LandingZone, step by step

The rule is `.claude/rules/lz-sync.md`; this is what is executed, usually once a month and
always before an `N` milestone is marked met. LZ is read, never written.

1. `just lz-sync`: fetches `reference/emibsd-lz`, then prints the LZ commits after the pin that
   have no record in `lz-sync.toml`, security candidates first, with the native modules and the
   items (`--functions`) each one touches.
2. Triage every commit into `lz-sync.toml`, one `[[commit]]` record each:
   - `applied`, with `emibsd = "<12-hex>"` and `method`: `cherry-pick` (an inherited module:
     `cargo xtask lz patch <hash> | git am -3`), `cherry-pick-conflicts` (an
     adapted module: conflicts expected at its call sites, resolved by hand) or `reimplemented`
     (a redesigned module: the change re-expressed in its new shape);
   - `not-applicable`, with the reason (nothing native derives from it, or the path does not
     exist here; since decision 57 (Limine removed) an LZ commit on `sys/stand/{mod,limine,fdtfb}.rs`
     or `limine.conf` is `not-applicable`, while one on `sys/stand/bootarg.rs` still applies);
   - `covered-by-redesign`, with the test or the type that proves it;
   - `deferred-to-N10` (decision 44), for a commit that touches only frozen amd64 files and does
     not cherry-pick cleanly; one that does is `applied` by `cherry-pick` without compiling,
     with `frozen = true`;
   - `security = true` on every security fix; such a record is never `not-applicable` without a
     reason that names the fix.
   The paths (the user's layout of 2026-11-26): LZ's `sys/` is flat, this tree keeps LZ's
   files in `sys/bsd/` (LZ's `sys/<x>`) and the loaders in `sys/boot/` (LZ's
   `sys/lib/libsa/`, `sys/stand/{efi,boot}/`, `sys/arch/<arch>/stand/<dir>/` are
   `sys/boot/libsa/`, `efi/`, `boot/`, `<arch>/<dir>/`). A plain `git format-patch | git am`
   or `git cherry-pick` would write LZ's paths; `cargo xtask lz patch <hash>` prints the
   commit's patch with every file header in the native layout (the message and the hunks
   LZ's), so `cargo xtask lz patch <hash> | git am -3` applies it where the files are, one
   commit with LZ's message, reworded to `lz-sync: apply LZ <12-hex> (...)` after. A header it
   cannot place (LZ's `sys/lib/libkern/`, `sys/lib/libz/` or `sys/sys/` file whose native home
   is a leaf crate's group, `sys/lib/<crate>/<group>/`) fails in `git am`, and that file is
   applied by hand as before. Check after the apply that no new file appeared outside the
   native directories (`git status`: nothing new under `sys/kern/`, `sys/dev/`, ... at the
   top of `sys/`). `modules` names native modules by LZ's path (`sys/kern/x.rs`) or by their
   own (`sys/bsd/kern/x.rs`): both read the same (`lz_flat`, `lineage.md`).
   A file new in LZ becomes a new `inherited` module of `lineage.toml` in the same commit,
   under its native path (`sys/bsd/...` or `sys/boot/...`).
   `modules` names every native module the commit touches, the new ones included: `lz check`
   compares each with the newest applied commit naming it, not with the pin, until the pin
   catches up.
3. One `lz-sync:` commit per LZ commit or coherent cluster, nothing else in it, trailer
   `LZ: <12-hex>`; the body carries the three method totals so far.
4. `just ci` (which runs `cargo xtask lz drift --strict`), then `just diff-openbsd`.
5. When every commit up to a point has a record, with the user's OK: `lz: bump pin to <12-hex>`
   (`lz/PINNED.md`, `lineage.toml [meta].lz`, `lz-sync.toml [meta].lz` together).
6. One line in `docs/JOURNAL.md`'s current section: the LZ range absorbed, the three method
   totals and the wall time.
