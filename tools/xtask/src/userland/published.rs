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
//! The files the ramdisk takes from EmiBSD/Userland's published set (N6.8): netstat(1) and
//! the libkvm.so it needs, which this build does not make (`tools/xtask/userland-set.toml`
//! says why and pins them).
//!
//! The set's tarball (`$EMIBSD_USERLAND_DIST/<tarball>`) is checked against its pinned
//! SHA-256; the base set inside it is taken out alone and checked against `base_sha256`;
//! then only the pinned `files` are taken out of the base set, each must be a regular file
//! and an ELF for this machine, and they are kept in `<out>/userland-set/<release>/`, where a
//! later build finds them again (a `done` marker names both hashes). [`stage`] copies them
//! into the ramdisk's staging tree and returns their owners and modes (root:bin, the pinned
//! mode). They go into the ramdisk only, never into `root/` or the install sets.
//!
//! Every extraction runs in a directory made empty for it, and the host's `tar` is given the
//! member names, so nothing else of the set is written.

use super::*;
use serde::Deserialize;

/// The pin file, relative to the workspace root.
const SET_FILE: &str = "tools/xtask/userland-set.toml";

/// The environment variable that names Userland's `dist/` directory.
const DIST_ENV: &str = "EMIBSD_USERLAND_DIST";

/// `tools/xtask/userland-set.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    release: String,
    arch: BTreeMap<String, ArchPin>,
}

/// One architecture's tarball, its base set and the files taken from it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchPin {
    tarball: String,
    sha256: String,
    base: String,
    base_sha256: String,
    files: Vec<PinnedFile>,
}

/// A file of the base set: its path (relative, in the set and in the image) and mode.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PinnedFile {
    path: String,
    mode: u32,
}

/// Whether `rel` is a plain relative path: no leading `/`, no empty, `.` or `..` component.
fn plain(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.starts_with('/')
        && rel
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
}

/// Whether `name` is a plain file name (no `/`, not `.` or `..`).
fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && name != "." && name != ".."
}

/// Takes `members` (paths as the archive names them) out of the `.tar.zst` `file` into the
/// empty directory `dir`: `zstd -dc` into `tar -x`.
fn untar_members(file: &Path, dir: &Path, members: &[String]) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut zstd = Command::new("zstd")
        .args(["-dc", "--"])
        .arg(file)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("zstd: {e}"))?;
    let Some(stdout) = zstd.stdout.take() else {
        return Err("zstd: no stdout".into());
    };
    let status = Command::new("tar")
        .arg("-xf")
        .arg("-")
        .arg("-C")
        .arg(dir)
        .args(members)
        .stdin(stdout)
        .status()
        .map_err(|e| format!("tar: {e}"))?;
    let zs = zstd.wait().map_err(|e| format!("zstd: {e}"))?;
    if !status.success() || !zs.success() {
        return Err(format!(
            "{}: taking {} out failed (zstd {zs}, tar {status})",
            file.display(),
            members.join(" ")
        )
        .into());
    }
    Ok(())
}

/// The regular file `p` (never a link), refused otherwise.
fn regular(p: &Path) -> Result<()> {
    let md = fs::symlink_metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
    if !md.file_type().is_file() {
        return Err(format!("{}: not a regular file in the set", p.display()).into());
    }
    Ok(())
}

/// Refuses `p` unless it is an ELF64 little-endian file for `e_machine`.
fn elf_for(p: &Path, e_machine: u16) -> Result<()> {
    let b = fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let ok = b.len() >= 20
        && b[..4] == *b"\x7fELF"
        && b[4] == 2
        && b[5] == 1
        && u16::from_le_bytes([b[18], b[19]]) == e_machine;
    if !ok {
        return Err(format!("{}: not an ELF64 for machine {e_machine}", p.display()).into());
    }
    Ok(())
}

/// The pinned files of `ctx`'s machine, taken out of Userland's set once and kept under
/// `<out>/userland-set/<release>/files`; `None` when the pin has no tarball for it.
fn fetch(ctx: &Ctx<'_>) -> Result<Option<(PathBuf, String, Vec<PinnedFile>)>> {
    let p = ctx.root.join(SET_FILE);
    let text = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut pin: Pin = toml::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))?;
    let Some(a) = pin.arch.remove(ctx.m.machine) else {
        return Ok(None);
    };
    if !plain_name(&pin.release) || !plain_name(&a.tarball) || !plain_name(&a.base) {
        return Err(format!("{SET_FILE}: release, tarball and base must be plain names").into());
    }
    for f in &a.files {
        if !plain(&f.path) {
            return Err(format!("{SET_FILE}: {}: not a plain relative path", f.path).into());
        }
    }
    let keep = ctx.out.join("userland-set").join(&pin.release);
    let files = keep.join("files");
    let marker = keep.join("done");
    let stamp = format!(
        "{} {}\n{}\n",
        a.sha256,
        a.base_sha256,
        a.files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    if fs::read_to_string(&marker).ok().as_deref() != Some(stamp.as_str()) {
        let dist = std::env::var_os(DIST_ENV).ok_or_else(|| {
            format!("${DIST_ENV} is not set (the justfile's `userland_dist`: Userland's dist/)")
        })?;
        let tarball = PathBuf::from(dist).join(&a.tarball);
        if !tarball.is_file() {
            return Err(format!(
                "{}: not found; the ramdisk takes netstat from Userland's release {} \
                 ({SET_FILE})",
                tarball.display(),
                pin.release
            )
            .into());
        }
        comp::check_sha256(&tarball, &a.sha256)?;
        // The base set alone, out of the release's directory in the tarball.
        let outer = keep.join("outer");
        let top = format!("{}/{}", a.tarball.trim_end_matches(".tar.zst"), a.base);
        untar_members(&tarball, &outer, std::slice::from_ref(&top))?;
        let base = outer.join(&top);
        regular(&base)?;
        comp::check_sha256(&base, &a.base_sha256)?;
        let members: Vec<String> = a.files.iter().map(|f| format!("./{}", f.path)).collect();
        untar_members(&base, &files, &members)?;
        for f in &a.files {
            let got = files.join(&f.path);
            regular(&got)?;
            elf_for(&got, ctx.m.e_machine)?;
        }
        fs::remove_dir_all(&outer).map_err(|e| format!("{}: {e}", outer.display()))?;
        fs::write(&marker, &stamp).map_err(|e| format!("{}: {e}", marker.display()))?;
    }
    Ok(Some((files, pin.release, a.files)))
}

/// Copies the pinned files into the ramdisk's `staging` tree and returns their owners and
/// modes; none (and a line that says so) when the pin has no set for this machine.
pub(super) fn stage(ctx: &Ctx<'_>, staging: &Path) -> Result<Vec<ramdisk::Attr>> {
    let Some((files, release, pinned)) = fetch(ctx)? else {
        println!(
            "  userland-set: no [arch.{}] in {SET_FILE}; nothing taken",
            ctx.m.machine
        );
        return Ok(Vec::new());
    };
    let bin = ramdisk::group_id("bin").ok_or("the image has no group `bin`")?;
    let mut attrs = Vec::new();
    for f in &pinned {
        let to = staging.join(&f.path);
        if to.exists() {
            return Err(format!(
                "{}: the build already has it; {SET_FILE} must not pin it",
                f.path
            )
            .into());
        }
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::copy(files.join(&f.path), &to).map_err(|e| format!("{}: {e}", to.display()))?;
        attrs.push(ramdisk::Attr::root(&format!("/{}", f.path), bin, f.mode));
    }
    println!(
        "  userland-set: {} from Userland {release} ({SET_FILE})",
        pinned
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(attrs)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paths_only() {
        assert!(plain("usr/bin/netstat"));
        assert!(!plain("/usr/bin/netstat"));
        assert!(!plain("usr/../etc/passwd"));
        assert!(!plain("usr//bin"));
        assert!(!plain("./usr/bin"));
        assert!(!plain(""));
        assert!(plain_name("base80.tar.zst"));
        assert!(!plain_name("../x"));
        assert!(!plain_name(".."));
    }

    #[test]
    fn the_pin_file_parses_and_names_plain_paths() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = fs::read_to_string(root.join(SET_FILE)).expect("the pin file");
        let pin: Pin = toml::from_str(&text).expect("it parses");
        let a = pin.arch.get("arm64").expect("arm64's set");
        assert_eq!(a.sha256.len(), 64);
        assert_eq!(a.base_sha256.len(), 64);
        assert!(a.files.iter().all(|f| plain(&f.path)));
        assert!(a.files.iter().any(|f| f.path == "usr/bin/netstat"));
    }
}
/* </TESTS> */
