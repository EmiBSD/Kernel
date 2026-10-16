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
//! The v2 bookkeeping `cargo xtask lz check` validates besides the modules (docs/ZERO_UNSAFE.md
//! section 0): the optional `crate` key of `[[module]]` and `[[extra]]`, and `bridges.toml`.
//!
//! * `crate = "hal" | "kernel" | "abi"` names the crate a native file lives in and must agree
//!   with the path prefix (`sys/hal/` is `hal`, `sys/kernel/` is `kernel`, `sys/abi/` is
//!   `abi`). A wrong or unknown value is an error; a file under one of those prefixes without
//!   the key is only a warning, so the redesigners' commits stay green until they add it.
//! * `bridges.toml` at the root is optional. Each `[[bridge]]` row has `trait` (an identifier),
//!   `for` (the `.rs` files under `sys/` that call through it, which must exist) and `paid_by`
//!   (`N2`..`N9`, the milestone that removes the bridge). A duplicate trait, a malformed row or
//!   an unknown key is an error. `lz status` prints `bridges: <n> open (<by milestone>)`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::Result;

pub(crate) const BRIDGES_FILE: &str = "bridges.toml";

/// The crate a path under `sys/` belongs to by its prefix, if it is one of the v2 crates.
pub(crate) fn crate_of_path(rust: &str) -> Option<&'static str> {
    [
        ("sys/hal/", "hal"),
        ("sys/kernel/", "kernel"),
        ("sys/abi/", "abi"),
    ]
    .into_iter()
    .find(|(p, _)| rust.starts_with(p))
    .map(|(_, c)| c)
}

/// Check one entry's `crate` key against its path: `(errors, warnings)`.
pub(crate) fn check_crate_key(
    what: &str,
    rust: &str,
    key: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let (mut errors, mut warnings) = (Vec::new(), Vec::new());
    let want = crate_of_path(rust);
    match (key, want) {
        (Some(k), _) if !matches!(k, "hal" | "kernel" | "abi") => errors.push(format!(
            "{what} rust = \"{rust}\": crate = \"{k}\" is not one of hal, kernel, abi"
        )),
        (Some(k), Some(w)) if k != w => errors.push(format!(
            "{what} rust = \"{rust}\": crate = \"{k}\" but the path is in `{w}`"
        )),
        (Some(k), None) => errors.push(format!(
            "{what} rust = \"{rust}\": crate = \"{k}\" but the path is in none of sys/hal/, sys/kernel/, sys/abi/"
        )),
        (None, Some(w)) => warnings.push(format!(
            "{what} rust = \"{rust}\": no `crate = \"{w}\"` key"
        )),
        _ => {}
    }
    (errors, warnings)
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bridge {
    #[serde(rename = "trait")]
    pub(crate) trait_name: String,
    #[serde(rename = "for")]
    pub(crate) users: Vec<String>,
    pub(crate) paid_by: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct BridgeFile {
    #[serde(default, rename = "bridge")]
    bridges: Vec<Bridge>,
}

/// Parse `bridges.toml` text.
pub(crate) fn parse_bridges(text: &str) -> Result<Vec<Bridge>> {
    let f: BridgeFile = toml::from_str(text).map_err(|e| format!("{BRIDGES_FILE}: {e}"))?;
    Ok(f.bridges)
}

/// The bridges of the tree; none when the file is absent.
pub(crate) fn load_bridges(root: &Path) -> Result<Vec<Bridge>> {
    let path = root.join(BRIDGES_FILE);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_bridges(&text)
}

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(x) if x.is_ascii_alphabetic() || x == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

fn is_milestone(s: &str) -> bool {
    s.len() == 2
        && matches!(
            s.strip_prefix('N').and_then(|d| d.parse::<u8>().ok()),
            Some(2..=9)
        )
}

/// Errors of a bridge list; `exists` answers whether a `for` path is a file of the tree.
pub(crate) fn check_bridges(bridges: &[Bridge], exists: impl Fn(&str) -> bool) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for b in bridges {
        let t = b.trait_name.as_str();
        if !is_ident(t) {
            errors.push(format!(
                "{BRIDGES_FILE}: trait \"{t}\" is not an identifier"
            ));
        }
        if seen.contains(&t) {
            errors.push(format!("{BRIDGES_FILE}: trait \"{t}\" appears twice"));
        }
        seen.push(t);
        if !is_milestone(&b.paid_by) {
            errors.push(format!(
                "{BRIDGES_FILE}: trait \"{t}\": paid_by = \"{}\" is not N2..N9",
                b.paid_by
            ));
        }
        if b.users.is_empty() {
            errors.push(format!("{BRIDGES_FILE}: trait \"{t}\": `for` is empty"));
        }
        for p in &b.users {
            if !p.starts_with("sys/") || !p.ends_with(".rs") || !exists(p) {
                errors.push(format!(
                    "{BRIDGES_FILE}: trait \"{t}\": for = \"{p}\" is not a .rs file under sys/"
                ));
            }
        }
    }
    errors
}

/// `bridges: <n> open (<by milestone>)`, e.g. `bridges: 3 open (N2 1, N4 2)`.
pub(crate) fn summary(bridges: &[Bridge]) -> String {
    let mut by: BTreeMap<&str, usize> = BTreeMap::new();
    for b in bridges {
        *by.entry(b.paid_by.as_str()).or_insert(0) += 1;
    }
    let parts: Vec<String> = by.iter().map(|(m, n)| format!("{m} {n}")).collect();
    if parts.is_empty() {
        "bridges: 0 open".to_string()
    } else {
        format!("bridges: {} open ({})", bridges.len(), parts.join(", "))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_key_agrees_with_the_path() {
        let ok = |rust, key| check_crate_key("[[module]]", rust, key);
        assert_eq!(ok("sys/hal/lock.rs", Some("hal")), (vec![], vec![]));
        assert_eq!(ok("sys/kernel/pipe.rs", Some("kernel")), (vec![], vec![]));
        assert_eq!(ok("sys/abi/sys/errno.rs", Some("abi")), (vec![], vec![]));
        assert_eq!(ok("sys/kern/kern_sig.rs", None), (vec![], vec![]));
    }

    #[test]
    fn a_missing_key_warns_and_a_wrong_one_errors() {
        let (e, w) = check_crate_key("[[module]]", "sys/hal/lock.rs", None);
        assert!(e.is_empty() && w.len() == 1, "{e:?} {w:?}");
        let (e, w) = check_crate_key("[[extra]]", "sys/hal/lock.rs", Some("kernel"));
        assert!(e.len() == 1 && w.is_empty(), "{e:?} {w:?}");
        let (e, _) = check_crate_key("[[module]]", "sys/kern/x.rs", Some("hal"));
        assert_eq!(e.len(), 1);
        let (e, _) = check_crate_key("[[module]]", "sys/hal/x.rs", Some("bsd"));
        assert_eq!(e.len(), 1);
    }

    const GOOD: &str = r#"
[[bridge]]
trait = "UvmKm"
for = ["sys/kernel/pipe.rs"]
paid_by = "N3"

[[bridge]]
trait = "VnodeOps"
for = ["sys/kernel/pipe.rs", "sys/kern/a.rs"]
paid_by = "N3"

[[bridge]]
trait = "Sched"
for = ["sys/kern/a.rs"]
paid_by = "N2"
"#;

    fn exists(p: &str) -> bool {
        matches!(p, "sys/kernel/pipe.rs" | "sys/kern/a.rs")
    }

    #[test]
    fn a_good_file_parses_and_summarises() {
        let b = parse_bridges(GOOD).unwrap();
        assert_eq!(b.len(), 3);
        assert!(check_bridges(&b, exists).is_empty());
        assert_eq!(summary(&b), "bridges: 3 open (N2 1, N3 2)");
        assert_eq!(summary(&[]), "bridges: 0 open");
    }

    #[test]
    fn unknown_keys_and_missing_fields_are_errors() {
        assert!(
            parse_bridges("[[bridge]]\ntrait = \"A\"\nfor = []\npaid_by = \"N2\"\nx = 1\n")
                .is_err()
        );
        assert!(parse_bridges("[[bridge]]\ntrait = \"A\"\n").is_err());
        assert!(parse_bridges("[meta]\nx = 1\n").is_err());
        assert!(parse_bridges("").unwrap().is_empty());
    }

    #[test]
    fn rows_are_validated() {
        let row = |t: &str, f: &str, p: &str| {
            parse_bridges(&format!(
                "[[bridge]]\ntrait = \"{t}\"\nfor = [{f}]\npaid_by = \"{p}\"\n"
            ))
            .unwrap()
        };
        let errs = |b: &[Bridge]| check_bridges(b, exists);
        assert!(errs(&row("A", "\"sys/kern/a.rs\"", "N9")).is_empty());
        assert_eq!(errs(&row("1A", "\"sys/kern/a.rs\"", "N2")).len(), 1);
        assert_eq!(errs(&row("A b", "\"sys/kern/a.rs\"", "N2")).len(), 1);
        for p in ["N1", "N10", "n3", "M3", ""] {
            assert_eq!(errs(&row("A", "\"sys/kern/a.rs\"", p)).len(), 1, "{p}");
        }
        assert_eq!(errs(&row("A", "", "N2")).len(), 1);
        assert_eq!(errs(&row("A", "\"sys/kern/missing.rs\"", "N2")).len(), 1);
        assert_eq!(errs(&row("A", "\"sys/kern/a.rs.txt\"", "N2")).len(), 1);
        assert_eq!(errs(&row("A", "\"lineage.toml\"", "N2")).len(), 1);
        let mut dup = row("A", "\"sys/kern/a.rs\"", "N2");
        dup.extend(row("A", "\"sys/kern/a.rs\"", "N3"));
        assert_eq!(errs(&dup).len(), 1);
    }
}
/* </TESTS> */
