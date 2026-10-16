#!/usr/bin/env python3
#
# Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
#
# Permission to use, copy, modify, and distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
#
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
# OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
#
"""Redesign progress of EmiBSD, one row per milestone of docs/ROADMAP.md.

Adapted from EmiBSD.LZ's .claude/skills/progress/scripts/progress.py (b0901dd3), which
measured the port from ports.toml; this one measures the zero-unsafe redesign
(docs/ZERO_UNSAFE.md section 0: the kernel crate with no unsafe) from
`cargo xtask unsafe-report --json`.

Sources, all read live (nothing is typed in by hand):
  - `cargo xtask unsafe-report --json`, run at the repository root: every kernel
    file with its lineage.toml status, its kernel `unsafe` and whether the tool
    counts it as a module; the totals; the ratchet with its baseline; the pace.
    unsafe-report is the one parser of the Rust sources: this script reads none.
    The crate of a file (docs/ZERO_UNSAFE.md section 0) is derived here from its
    path: sys/hal/, sys/kernel/, sys/abi/, the boot loaders (sys/stand/,
    sys/lib/libsa/), and bsd (the LZ tree, legacy) for the rest of sys/;
  - docs/ROADMAP.md: the milestone table (id, title, scope, exit criterion). A
    file belongs to the N row whose scope cell names its path in backticks: a
    directory (`sys/uvm/`), a file (`sys/sys/queue.rs`) or a glob
    (`sys/kern/vfs_*`); when several rows name it, the most specific pattern
    wins. A parent row (N2 with N2a, ...) adds its lettered children's;
  - bridges.toml: the bridges open, by the milestone that pays them (`paid_by`);
  - lineage.toml and reference/emibsd-lz at the pin: each module's LZ sources and
    their lines (`git cat-file --batch`), for the module list only;
  - git: the `docs: Nx met` close commits.

Columns, per row (docs/ZERO_UNSAFE.md 0.4):
  Files in bsd      the row's files still in the LZ tree (counted as modules, or
                    holding unsafe): the exit criterion's first number;
  Unsafe in bsd     their kernel `unsafe`: the exit criterion's second number;
  Moved             the row's files already in hal, kernel or abi;
  Bridges open      bridges.toml rows whose paid_by is this row;
  Redesigned        lineage.toml's redesigned modules, adapted shown apart;
  Est. left         Unsafe in bsd / the pace; "done" once Files in bsd, Unsafe in
                    bsd and Bridges open are all 0; "n/a" until 5 active days since
                    the baseline have removed 100 or more (MIN_PACE_DAYS,
                    MIN_PACE_REMOVED): a smaller sample is noise.
The Crates line gives kernel, abi, hal (with its ceiling when unsafe-budget.toml
records hal_ceiling), bsd and the boot loaders; a file in bsd no row names is a
warning line.

Without cargo (or if unsafe-report fails) the script prints, with a note, the
lineage table it printed before the unsafe metric (modules redesigned and
inherited, LZ lines, the LZ-lines rate since lz-origin).

The estimate is a linear extrapolation of the project's own pace. It measures the
past; it promises nothing.

Usage: progress.py [MILESTONE]   (e.g. N2: adds that milestone's file list)
Always exits 0; errors are printed, so a skill preamble never aborts on them.
"""

import datetime as _dt
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

if sys.version_info < (3, 11):  # macOS's /usr/bin/python3 is 3.9: re-exec under a newer one
    import os
    import shutil

    for name in ("python3.14", "python3.13", "python3.12", "python3.11"):
        exe = shutil.which(name) or shutil.which(name, path="/opt/homebrew/bin:/usr/local/bin")
        if exe:
            os.execv(exe, [exe, "-I", __file__, *sys.argv[1:]])
    print(f"progress: Python 3.11+ is needed (tomllib); this is {sys.version.split()[0]}")
    sys.exit(0)

import tomllib  # noqa: E402

MS_ID = re.compile(r"N\d+[a-z]*")
STATUSES = ("redesigned", "adapted", "inherited")
DONE = ("redesigned", "adapted")
# The smallest sample the pace is computed from: below it, Est. left reads n/a.
MIN_PACE_DAYS = 5
MIN_PACE_REMOVED = 100


def root_dir() -> Path:
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env and (Path(env) / "lineage.toml").is_file():
        return Path(env)
    for p in Path(__file__).resolve().parents:
        if (p / "lineage.toml").is_file() and (p / "docs" / "ROADMAP.md").is_file():
            return p
    raise SystemExit("progress: cannot find the repository root (lineage.toml)")


def count_lines(path: Path, cache: dict) -> int | None:
    if path in cache:
        return cache[path]
    try:
        with open(path, "rb") as f:
            n = f.read().count(b"\n")
    except OSError:
        n = None
    cache[path] = n
    return n


# --- ROADMAP ---------------------------------------------------------------

def parse_roadmap(root: Path) -> list[dict]:
    """The milestone table: [{id, title, scope, criterion, row_date}] in file order."""
    rows = []
    for line in (root / "docs" / "ROADMAP.md").read_text().splitlines():
        if not line.startswith("| **N"):
            continue
        cells = [c.strip() for c in re.split(r"(?<!\\)\|", line)][1:-1]
        if len(cells) < 3:
            continue
        m = re.match(r"\*\*(N\d+[a-z]*)\s*(.*?)\*\*", cells[0])
        if not m:
            continue
        dm = re.search(r"\b(?:done|Met|met)\**\s+(\d{4}-\d{2}-\d{2})", line)
        rows.append({"id": m.group(1), "title": m.group(2).strip(), "scope": cells[1],
                     "criterion": cells[2], "row_date": dm.group(1) if dm else None})
    return rows


def scope_patterns(scope: str) -> list[str]:
    """The `sys/...` (or `init/`, the init stand-in) paths a scope cell names in backticks: a
    directory, a file or a glob."""
    pats = []
    for tok in re.findall(r"`([^`]+)`", scope):
        tok = tok.strip()
        if tok.startswith(("sys/", "init/")) and " " not in tok and "{" not in tok:
            pats.append(tok)
    return pats


def pattern_matches(pattern: str, path: str) -> bool:
    if any(ch in pattern for ch in "*?["):
        return fnmatch.fnmatchcase(path, pattern)
    if pattern.endswith("/"):
        return path.startswith(pattern)
    if pattern.endswith(".rs"):
        return path == pattern
    return path == pattern or path.startswith(pattern + "/")


def specificity(pattern: str) -> int:
    """The literal prefix length: `sys/kern/vfs_*` beats `sys/kern/`."""
    m = re.search(r"[*?\[]", pattern)
    return len(pattern) if m is None else m.start()


# --- lineage.toml ------------------------------------------------------------

def parse_lineage(root: Path) -> tuple[dict, list[dict], list[dict]]:
    data = tomllib.loads((root / "lineage.toml").read_text())
    return data.get("meta", {}), data.get("module", []), data.get("extra", [])


# --- unsafe-report ---------------------------------------------------------------

CARGO_DIRS = ("/opt/homebrew/opt/rustup/bin", "/usr/local/opt/rustup/bin",
              str(Path.home() / ".cargo" / "bin"))


def unsafe_json(root: Path) -> tuple[dict | None, str]:
    """`cargo xtask unsafe-report --json` at the root: (the document, "") or (None, why not)."""
    cargo = shutil.which("cargo") or shutil.which("cargo", path=os.pathsep.join(CARGO_DIRS))
    if not cargo:
        return None, "cargo not found (PATH, rustup's bin, ~/.cargo/bin)"
    env = dict(os.environ)
    env["PATH"] = os.pathsep.join([str(Path(cargo).parent), env.get("PATH", "")])
    try:
        r = subprocess.run([cargo, "xtask", "unsafe-report", "--json"], cwd=root, env=env,
                           capture_output=True, text=True, timeout=900)
    except (OSError, subprocess.TimeoutExpired) as e:
        return None, f"cargo xtask unsafe-report --json: {e}"
    if r.returncode != 0:
        tail = (r.stderr.strip().splitlines() or ["no output"])[-1]
        return None, f"cargo xtask unsafe-report --json failed: {tail}"
    try:
        doc = json.loads(r.stdout)
    except json.JSONDecodeError as e:
        return None, f"cargo xtask unsafe-report --json printed no JSON: {e}"
    if doc.get("schema") != 1:
        return None, f"cargo xtask unsafe-report --json: schema {doc.get('schema')!r}, not 1"
    return doc, ""


# --- git ---------------------------------------------------------------------

def git(root: Path, *args: str) -> str:
    try:
        return subprocess.run(["git", "-C", str(root), *args], check=True,
                              capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"progress: git {' '.join(args)} failed: {e}")
        return ""


def close_dates(root: Path, rng: str) -> dict[str, str]:
    """Milestone id -> the date it was declared met (`docs: Nx met ...` in the range)."""
    dates: dict[str, str] = {}
    for line in reversed(git(root, "log", rng, "--format=%cd%x09%s", "--date=short").splitlines()):
        date, _, subject = line.partition("\t")
        if not subject.startswith("docs:") or not re.search(r"\bmet\b", subject):
            continue
        if re.search(r"\b(left|blocked|partial|pending)\b", subject):
            continue  # a partial criterion, not the close
        for mid in MS_ID.findall(subject.split("(")[0]):
            dates.setdefault(mid, date)
    return dates


def under_way(root: Path) -> set[str]:
    status = root / "docs" / "STATUS.md"
    if not status.is_file():
        return set()
    return {m.group(1) for m in
            re.finditer(r"(N\d+[a-z]*)\b[^.;]*?\bunder way", status.read_text())}


def lz_lines(root: Path, pin: str, paths: list[str]) -> dict[str, int | None] | None:
    """LZ path -> lines of `<pin>:<path>` in reference/emibsd-lz; None when the clone is absent."""
    clone = root / "reference" / "emibsd-lz"
    if not pin or not paths or not clone.is_dir():
        return None
    req = "".join(f"{pin}:{p}\n" for p in paths).encode()
    try:
        r = subprocess.run(["git", "-C", str(clone), "cat-file", "--batch"], input=req,
                           capture_output=True, check=True)
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"progress: git cat-file in reference/emibsd-lz failed: {e}")
        return None
    out: dict[str, int | None] = {}
    data, pos = r.stdout, 0
    for p in paths:
        nl = data.find(b"\n", pos)
        if nl < 0:
            out[p] = None
            continue
        header = data[pos:nl].decode(errors="replace").split()
        pos = nl + 1
        if len(header) == 3 and header[1] == "blob":  # "<sha> blob <size>"
            size = int(header[2])
            out[p] = data[pos:pos + size].count(b"\n")
            pos += size + 1  # the trailing LF
        else:  # "<name> missing"
            out[p] = None
    return out


# --- main -----------------------------------------------------------------------

def fmt(n) -> str:
    return "—" if n is None else f"{n:,}"


def subsystem(path: str) -> str:
    parts = path.split("/")
    if len(parts) > 3 and parts[1] in ("arch", "dev", "lib"):
        return parts[1] + "/" + parts[2]
    return parts[1] if len(parts) > 2 else parts[0]


def lz_line_counts(root: Path, pin: str, modules: list[dict], cache: dict) -> tuple[dict, bool]:
    """LZ path -> its lines at the pin, for the LZ sources of `modules`; and whether the clone is
    missing (then the native file of an inherited module stands in: it is LZ's, byte for byte)."""
    lz_paths = sorted({p for m in modules for p in m.get("lz", [])})
    lz_len = lz_lines(root, pin, lz_paths)
    if lz_len is not None:
        return lz_len, False
    lz_len = {}
    for m in modules:
        if m.get("status", "inherited") == "inherited" and len(m.get("lz", [])) == 1:
            lz_len[m["lz"][0]] = count_lines(root / m["rust"], cache)
    return lz_len, True


class Plan:
    """The ROADMAP rows: who owns a path, the lettered children, each row's status."""

    def __init__(self, root: Path, rows: list[dict], pin: str):
        self.rows = rows
        self.ids = [r["id"] for r in rows]
        self.children = {rid: [c for c in self.ids if re.fullmatch(re.escape(rid) + r"[a-z]", c)]
                         for rid in self.ids}
        self.is_child = {c for cs in self.children.values() for c in cs}
        self.pats = []  # (specificity, -row index, pattern, row id)
        for i, r in enumerate(rows):
            for p in scope_patterns(r["scope"]):
                self.pats.append((specificity(p), -i, p, r["id"]))
        self.in_history = bool(pin) and subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", pin, "HEAD"],
            capture_output=True).returncode == 0
        self.rng = f"{pin}..HEAD" if self.in_history else "HEAD"
        self.dates = close_dates(root, self.rng)
        self.wip = under_way(root)

    def owner(self, path: str) -> str:
        """The row whose scope names the path, the most specific pattern winning."""
        best = None
        for spec, negi, p, rid in self.pats:
            if pattern_matches(p, path) and (best is None or (spec, negi) > best[:2]):
                best = (spec, negi, rid)
        return best[2] if best else "unassigned"

    def selected(self, rid: str, want: str | None) -> bool:
        """The row is shown: no milestone was asked, or it is the one asked or its child."""
        return not want or rid == want or bool(re.fullmatch(re.escape(want) + r"[a-z]", rid))

    def row(self, rid: str) -> dict:
        return next(x for x in self.rows if x["id"] == rid)

    def has_patterns(self, rid: str) -> bool:
        return bool(scope_patterns(self.row(rid)["scope"])) or bool(self.children[rid])

    def aggregate(self, own: dict) -> dict:
        """A parent row (N2 with N2a, N2b, ...) shows its own tally plus its children's."""
        agg = {}
        for rid, t in own.items():
            a = {k: (set(v) if isinstance(v, set) else v) for k, v in t.items()}
            for cid in self.children.get(rid, []):
                for k, v in own[cid].items():
                    if isinstance(v, set):
                        a[k] |= v
                    else:
                        a[k] += v
            agg[rid] = a
        return agg

    def status(self, rid: str, finished, busy) -> str:
        """met (its close commit, or its ROADMAP date once finished), under way or pending."""
        kids = self.children[rid]
        if kids and all(self.status(c, finished, busy).startswith("met") for c in kids):
            return "met " + (self.dates.get(rid) or max(self.dates.get(c, "") for c in kids))
        if rid in self.dates:
            return "met " + self.dates[rid]
        if self.row(rid)["row_date"] and finished(rid):
            return "met " + self.row(rid)["row_date"]
        if rid in self.wip or busy(rid):
            return "under way"
        return "pending"


CRATES = (("hal", "sys/hal/"), ("kernel", "sys/kernel/"), ("abi", "sys/abi/"),
          ("boot", "sys/stand/"), ("boot", "sys/lib/libsa/"))


def crate_of(path: str) -> str:
    """The crate a file belongs to under docs/ZERO_UNSAFE.md section 0, by its path: hal,
    kernel, abi, boot (the loaders, outside the four crates) or bsd (the LZ tree, legacy)."""
    for name, prefix in CRATES:
        if path.startswith(prefix):
            return name
    return "bsd"


def bridges(root: Path) -> list[dict]:
    """The rows of bridges.toml (trait, for, paid_by); none while the file does not exist."""
    try:
        return tomllib.loads((root / "bridges.toml").read_text()).get("bridge", [])
    except (OSError, tomllib.TOMLDecodeError):
        return []


def unsafe_report(root: Path, want: str | None, doc: dict, plan: Plan, modules: list[dict],
                  pin: str) -> None:
    """The zero-unsafe table of docs/ZERO_UNSAFE.md 0.4, computed by path from
    `cargo xtask unsafe-report --json`: what is still in bsd per milestone, what moved into
    the crates, the bridges open, and the crates' totals."""
    cache: dict = {}
    files = doc["files"]
    totals = doc["totals"]
    ratchet = doc.get("ratchet") or {}
    pace = doc.get("rate") or {}
    per_day = pace.get("per_day")
    days = pace.get("active_days", 0)
    # A pace from a handful of days or a handful of unsafe extrapolates noise (2026-10-16: one
    # unsafe in two planning days gave "~232 d" for N2); below this sample there is no estimate.
    if days < MIN_PACE_DAYS or pace.get("removed", 0) < MIN_PACE_REMOVED:
        per_day = None
    base, since = ratchet.get("legacy_baseline"), ratchet.get("baseline_date")
    for f in files:
        f["crate"] = crate_of(f["path"])
    br = bridges(root)

    def tally() -> dict:
        return {"bsd_files": 0, "bsd_unsafe": 0, "moved": 0, "redesigned": 0, "adapted": 0}

    own = {rid: tally() for rid in plan.ids + ["unassigned"]}
    files_of: dict[str, list[dict]] = {rid: [] for rid in own}
    for f in files:
        rid = plan.owner(f["path"])
        files_of[rid].append(f)
        t = own[rid]
        counted = f["module"] or f["unsafe"]
        if f["crate"] == "bsd":
            if counted:
                t["bsd_files"] += 1
            t["bsd_unsafe"] += f["unsafe"]
        elif f["crate"] != "boot" and counted:
            t["moved"] += 1
        if f["status"] in ("redesigned", "adapted"):
            t[f["status"]] += 1
    agg = plan.aggregate(own)
    open_bridges = {rid: sum(1 for x in br if x.get("paid_by") == rid) for rid in plan.ids}

    def finished(rid: str) -> bool:
        """Complete: nothing of the row left in bsd (0.4), and no bridge it must pay."""
        return (agg[rid]["bsd_files"] == 0 and agg[rid]["bsd_unsafe"] == 0
                and open_bridges.get(rid, 0) == 0)

    def busy(rid: str) -> bool:
        return (agg[rid]["moved"] > 0 or agg[rid]["redesigned"] > 0) and not finished(rid)

    def est(rid: str) -> str:
        if finished(rid):
            return "done"
        if not per_day:
            return "n/a"
        d = agg[rid]["bsd_unsafe"] / per_day
        return "<0.1 d" if d < 0.1 else f"~{d:.1f} d"

    crate_unsafe = {c: 0 for c in ("kernel", "abi", "hal", "bsd", "boot")}
    crate_files = {c: 0 for c in crate_unsafe}
    for f in files:
        crate_unsafe[f["crate"]] += f["unsafe"]
        if f["module"] or f["unsafe"]:
            crate_files[f["crate"]] += 1
    hal_ceiling = ratchet.get("hal_ceiling")
    baseline = (f"; baseline {base:,} on {since}, {days} active day{'s' if days != 1 else ''} "
                "since" if base is not None and since else "")
    print(f"EmiBSD redesign progress — {_dt.date.today()} — EmiBSD.LZ pin {pin[:12] or '?'} — "
          f"docs/ZERO_UNSAFE.md v2 — bsd unsafe {crate_unsafe['bsd']:,}{baseline}")
    print()
    print("| Milestone | Title | Status | Files in bsd | Unsafe in bsd | Moved | Bridges open | "
          "Redesigned | Est. left |")
    print("|---|---|---|---:|---:|---:|---:|---:|---:|")
    for r in plan.rows:
        rid = r["id"]
        if not plan.selected(rid, want):
            continue
        t = agg[rid]
        if not plan.has_patterns(rid):
            cells = ["—"] * 5
        elif not (t["bsd_files"] or t["moved"]):
            cells = ["no files match"] + ["—"] * 4
        else:
            cells = [fmt(t["bsd_files"]), fmt(t["bsd_unsafe"]), fmt(t["moved"]),
                     fmt(open_bridges.get(rid, 0)),
                     fmt(t["redesigned"]) + (f" + {t['adapted']} adapted" if t["adapted"] else "")]
        indent = "&nbsp;&nbsp;" if rid in plan.is_child else ""
        print(f"| {indent}{rid} | {r['title']} | {plan.status(rid, finished, busy)} | "
              + " | ".join(cells) + f" | {est(rid)} |")
    print()
    print(f"Crates: kernel {crate_unsafe['kernel']:,} unsafe in {crate_files['kernel']:,} files; "
          f"abi {crate_unsafe['abi']:,} in {crate_files['abi']:,}; "
          f"hal {crate_unsafe['hal']:,} in {crate_files['hal']:,}"
          + (f" (ceiling {hal_ceiling:,})" if hal_ceiling is not None else "")
          + f"; bsd {crate_unsafe['bsd']:,} in {crate_files['bsd']:,}; "
          f"boot loaders {crate_unsafe['boot']:,} in {crate_files['boot']:,} (outside the plan). "
          f"Bridges open: {len(br)}.")
    stray = sorted(f["path"] for f in files_of["unassigned"]
                   if f["crate"] == "bsd" and (f["module"] or f["unsafe"]))
    if stray and not want:
        print(f"Warning: {len(stray)} file{'s' if len(stray) != 1 else ''} in bsd no ROADMAP "
              f"row names: {', '.join(stray[:10])}{', ...' if len(stray) > 10 else ''}.")
    print()

    if want:
        ids = [rid for rid in plan.ids if plan.selected(rid, want)]
        sel = [f for rid in ids for f in files_of.get(rid, []) if f["module"] or f["unsafe"]]
        if sel:
            by_path = {m.get("rust", ""): m for m in modules}
            lz_len, _ = lz_line_counts(
                root, pin, [by_path[f["path"]] for f in sel if f["path"] in by_path], cache)
            print(f"Files of {want} ({len(sel)}):")
            print()
            print("| File | Status | Crate | Unsafe | LZ lines | Rust lines | LZ sources |")
            print("|---|---|---|---:|---:|---:|---|")
            order = {"bsd": 0, "hal": 1, "kernel": 2, "abi": 3, "boot": 4}
            for f in sorted(sel, key=lambda f: (order[f["crate"]], -f["unsafe"], f["path"])):
                srcs = by_path.get(f["path"], {}).get("lz", [])
                vals = [lz_len.get(p) for p in srcs]
                lz_n = sum(vals) if vals and all(v is not None for v in vals) else None
                src_txt = "same path" if srcs == [f["path"]] else (", ".join(srcs) or "—")
                print(f"| {f['path']} | {f['status'] or '—'} | {f['crate']} | {f['unsafe']:,} | "
                      f"{fmt(lz_n)} | {fmt(count_lines(root / f['path'], cache))} | {src_txt} |")
            print()
        else:
            print(f"No ROADMAP row named {want}, or its scope names no file.")
            print()

    counts = {s: sum(1 for m in modules if m.get("status", "inherited") == s) for s in STATUSES}
    print(f"Totals: {sum(crate_files.values()):,} files; bsd {crate_unsafe['bsd']:,} unsafe "
          f"in {crate_files['bsd']:,} files (must reach 0 files at N9); kernel "
          f"{crate_unsafe['kernel']:,}; abi {crate_unsafe['abi']:,}; hal {crate_unsafe['hal']:,}; "
          f"lineage.toml: {counts['redesigned']} redesigned, {counts['adapted']} adapted, "
          f"{counts['inherited']} inherited.")
    if base is None or not since:
        print("Pace: n/a: unsafe-budget.toml's [ratchet] records no legacy_baseline and "
              "baseline_date.")
    elif per_day:
        print(f"Pace: {pace['removed']:,} unsafe removed over {days} active days since "
              f"{since} (baseline {base:,}, the v1 legacy count until N2.4's tool): "
              f"{per_day:,.1f} a day. All of bsd at that pace: "
              f"~{crate_unsafe['bsd'] / per_day:.1f} d.")
    else:
        print(f"Pace: n/a: {days} active day{'s' if days != 1 else ''} since the baseline "
              f"({since}, {base:,}), {pace.get('removed', 0):,} removed; the "
              f"estimate needs {MIN_PACE_DAYS} active days and {MIN_PACE_REMOVED} unsafe removed.")
    print()
    print("Method: one source, `cargo xtask unsafe-report --json`; this script reads no Rust. "
          "A file's crate comes from its path (docs/ZERO_UNSAFE.md section 0): sys/hal/, "
          "sys/kernel/, sys/abi/, the boot loaders (sys/stand/, sys/lib/libsa/, outside the "
          "plan), and bsd for everything else under sys/ (the LZ tree, legacy). A file belongs "
          "to the N row whose scope cell names its path in backticks (a directory, a file or a "
          "glob), the most specific pattern winning; a row whose scope is prose resolves "
          "nothing; a parent row adds its lettered children's. Files in bsd and Unsafe in bsd "
          "are the exit criterion: a row is complete when both are 0 and no bridge names it in "
          "`paid_by` (bridges.toml). Moved = the row's files already in hal, kernel or abi. "
          "Redesigned = lineage.toml's redesigned modules, adapted shown apart. Est. left = "
          "Unsafe in bsd / the pace, the unsafe removed per active day (a day with a commit) "
          "since the ratchet's baseline; n/a before 5 active days and 100 unsafe removed. It is "
          "a linear extrapolation: it measures the past and promises nothing.")


def new_tally() -> dict:
    return {"redesigned": 0, "adapted": 0, "inherited": 0, "rust": 0,
            "lz_done": set(), "lz_left": set()}


def lineage_report(root: Path, want: str | None, plan: Plan, modules: list[dict],
                   extras: list[dict], pin: str, why: str) -> None:
    """The table this script printed before the unsafe metric, for when unsafe-report cannot
    run: modules redesigned and inherited, LZ lines, the LZ-lines rate since lz-origin."""
    print(f"Note: {why}. The unsafe metric needs `cargo xtask unsafe-report --json`; this is the "
          "lineage table instead.")
    print()
    cache: dict = {}
    commit_dates = sorted(set(git(root, "log", plan.rng, "--format=%cd", "--date=short").split()))
    days = len(commit_dates) or 1
    lz_len, clone_missing = lz_line_counts(root, pin, modules, cache)

    def lines_of(paths) -> tuple[int, int]:
        """(lines, files whose size is unknown) of a set of LZ paths."""
        total, unknown = 0, 0
        for p in paths:
            n = lz_len.get(p)
            if n is None:
                unknown += 1
            else:
                total += n
        return total, unknown

    own = {rid: new_tally() for rid in plan.ids + ["unassigned"]}
    rows_of: dict[str, list[dict]] = {rid: [] for rid in own}
    for m in modules:
        path = m.get("rust", "")
        st = m.get("status", "inherited")
        key = st if st in STATUSES else "inherited"
        rid = plan.owner(path)
        rows_of[rid].append(m)
        t = own[rid]
        t[key] += 1
        rl = count_lines(root / path, cache)
        m["_rust"] = rl
        m["_key"] = key
        if rl is not None:
            t["rust"] += rl
        srcs = m.get("lz", [])
        vals = [lz_len.get(p) for p in srcs]
        m["_lz"] = sum(vals) if vals and all(v is not None for v in vals) else None
        (t["lz_done"] if key in DONE else t["lz_left"]).update(srcs)
    for t in own.values():
        t["lz_left"] -= t["lz_done"]

    agg = plan.aggregate(own)
    for a in agg.values():
        a["lz_left"] -= a["lz_done"]

    total = new_tally()
    for t in own.values():
        for k, v in t.items():
            if isinstance(v, set):
                total[k] |= v
            else:
                total[k] += v
    total["lz_left"] -= total["lz_done"]
    done_lines, done_unknown = lines_of(total["lz_done"])
    left_lines, left_unknown = lines_of(total["lz_left"])
    rate = done_lines / days

    def status_of(rid: str) -> str:
        return plan.status(rid, lambda r: not agg[r]["inherited"],
                           lambda r: bool(agg[r]["redesigned"] and agg[r]["inherited"]))

    def est_days(lines: int) -> str:
        if not rate:
            return "—"
        d = lines / rate
        return "<0.1 d" if d < 0.1 else f"~{d:.1f} d"

    def with_unknown(n: int, unknown: int) -> str:
        return fmt(n) + (f" (+{unknown} unknown)" if unknown else "")

    print(f"EmiBSD redesign progress — {_dt.date.today()} — EmiBSD.LZ pin {pin[:12] or '?'} — "
          f"{len(modules)} modules ({len(extras)} extras not counted) — "
          f"{days} active day{'s' if days != 1 else ''} since lz-origin"
          f"{' (' + commit_dates[0] + ')' if commit_dates else ''}")
    print()
    print("| Milestone | Title | Status | Modules done | Modules left | LZ lines done | "
          "LZ lines left | Rust lines | Est. left |")
    print("|---|---|---|---:|---:|---:|---:|---:|---:|")
    for r in plan.rows:
        rid = r["id"]
        if not plan.selected(rid, want):
            continue
        t = agg[rid]
        st = status_of(rid)
        has_patterns = plan.has_patterns(rid)
        has_modules = any(t[k] for k in STATUSES)
        done_n = t["redesigned"] + t["adapted"]
        done_txt = (fmt(done_n) + (f" ({t['adapted']} adapted)" if t["adapted"] else "")) if has_modules else "—"
        left_txt = fmt(t["inherited"]) if has_modules else "—"
        ld, lu = lines_of(t["lz_done"])
        ll, llu = lines_of(t["lz_left"])
        if not has_patterns:
            lz_left_txt = "—" if st.startswith("met") else "scope in prose: not resolved"
            lz_done_txt = "—"
        elif not has_modules:
            lz_left_txt, lz_done_txt = "no modules match", "—"
        else:
            lz_done_txt, lz_left_txt = with_unknown(ld, lu), with_unknown(ll, llu)
        if st.startswith("met"):
            est = "done"
        elif ll:
            est = est_days(ll)
        else:
            est = "—"
        indent = "&nbsp;&nbsp;" if rid in plan.is_child else ""
        print(f"| {indent}{rid} | {r['title']} | {st} | {done_txt} | {left_txt} | "
              f"{lz_done_txt} | {lz_left_txt} | {fmt(t['rust']) if t['rust'] else '—'} | {est} |")
    u = own["unassigned"]
    if any(u[k] for k in STATUSES) and not want:
        subs = sorted({subsystem(m.get("rust", "")) for m in rows_of["unassigned"]})
        shown = ", ".join(subs[:8]) + (", ..." if len(subs) > 8 else "")
        ud, udu = lines_of(u["lz_done"])
        ul, ulu = lines_of(u["lz_left"])
        adapted_note = f" ({u['adapted']} adapted)" if u["adapted"] else ""
        print(f"| (no milestone) | modules no ROADMAP scope names: {shown} | — | "
              f"{fmt(u['redesigned'] + u['adapted'])}"
              f"{adapted_note} | {fmt(u['inherited'])} | "
              f"{with_unknown(ud, udu)} | {with_unknown(ul, ulu)} | {fmt(u['rust'])} | — |")
    print()

    if want:
        ids = [rid for rid in plan.ids if plan.selected(rid, want)]
        sel = [m for rid in ids for m in rows_of.get(rid, [])]
        if sel:
            print(f"Modules of {want} ({len(sel)}):")
            print()
            print("| Module | Status | LZ lines | Rust lines | LZ sources |")
            print("|---|---|---:|---:|---|")
            order = {"inherited": 0, "adapted": 1, "redesigned": 2}
            for m in sorted(sel, key=lambda m: (order[m["_key"]], m.get("rust", ""))):
                srcs = m.get("lz", [])
                src_txt = "same path" if srcs == [m.get("rust")] else ", ".join(srcs)
                print(f"| {m.get('rust', '')} | {m['_key']} | {fmt(m['_lz'])} | "
                      f"{fmt(m['_rust'])} | {src_txt} |")
            print()
        else:
            print(f"No ROADMAP row named {want}, or its scope names no module.")
            print()

    rust_total = 0
    for sub in ("sys", "tools"):
        for p in (root / sub).rglob("*.rs"):
            rust_total += count_lines(p, cache) or 0
    print(f"Totals: {total['redesigned']} modules redesigned, {total['adapted']} adapted, "
          f"{total['inherited']} inherited (of {len(modules)}); "
          f"LZ lines done {with_unknown(done_lines, done_unknown)}, left {with_unknown(left_lines, left_unknown)}; "
          f"Rust lines in sys/ and tools/: {rust_total:,}.")
    if rate:
        print(f"Rate: {rate:,.0f} LZ lines per active day over {days} active day{'s' if days != 1 else ''} "
              f"since lz-origin. Every inherited module, at that rate: {est_days(left_lines)}.")
    if clone_missing:
        print("Warning: reference/emibsd-lz is not checked out: inherited modules are measured on the "
              "native file (identical to LZ's); the LZ sources of redesigned modules count as unknown.")
    if not plan.in_history:
        print("Warning: the pinned LZ commit is not in this history; the rate counts every commit date.")
    print()
    print("Method: modules = lineage.toml [[module]] rows; done = redesigned, with adapted shown apart "
          "(their call sites followed a redesign, nothing of their own changed); left = inherited; "
          "extras count nowhere. A module belongs to the N row whose scope cell names its path in "
          "backticks (a directory, a file or a glob), the most specific pattern winning; a row whose "
          "scope is prose resolves nothing, and a met row may keep inherited modules that a later "
          "row's prose claims. LZ lines = lines of the module's LZ sources at the pin, each file once "
          "per row; Rust lines = wc -l of the native file, tests included. A parent milestone shows "
          "its own modules plus its lettered children's. The time column is a linear extrapolation "
          "of the project's own average; it measures the past and promises nothing.")


def main() -> None:
    root = root_dir()
    want = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else None
    meta, modules, extras = parse_lineage(root)
    pin = str(meta.get("lz", ""))
    plan = Plan(root, parse_roadmap(root), pin)
    doc, why = unsafe_json(root)
    if doc is None:
        lineage_report(root, want, plan, modules, extras, pin, why)
    else:
        unsafe_report(root, want, doc, plan, modules, pin)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:  # the skill preamble must never abort on this script
        print(f"progress: error: {exc!r}")
    sys.exit(0)
