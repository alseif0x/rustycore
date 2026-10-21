// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Which checkout a scan reads (#1468 item 1).
//!
//! The library finds the repository through `CARGO_MANIFEST_DIR`, a path fixed
//! at compile time. A prebuilt binary — or one another checkout rebuilt into a
//! shared `CARGO_TARGET_DIR` — therefore scans the tree it was compiled in, not
//! the one the operator is standing in, and a PASS says nothing about the
//! current checkout. The binaries resolve their scan root here instead: the
//! compiled tree only when it is the current directory's git top-level, or an
//! explicit `--root` the operator chose.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

static SCAN_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// The checkout this binary was compiled from.
pub(crate) fn compiled_repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .map_err(|error| format!("cannot resolve repository root: {error}"))
}

/// The root every scan reads: the one a binary selected, else the compiled
/// tree (library tests scan the tree they were compiled from).
pub(crate) fn repository_root() -> Result<PathBuf, String> {
    match SCAN_ROOT.get() {
        Some(root) => Ok(root.clone()),
        None => compiled_repository_root(),
    }
}

/// Remove one `--root PATH` option from `arguments`, wherever it appears.
pub fn take_root_argument(
    arguments: Vec<OsString>,
) -> Result<(Option<PathBuf>, Vec<OsString>), String> {
    let mut root = None;
    let mut rest = Vec::with_capacity(arguments.len());
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument != "--root" {
            rest.push(argument);
            continue;
        }
        if root.is_some() {
            return Err("--root is given more than once".to_owned());
        }
        let value = arguments
            .next()
            .ok_or_else(|| "--root requires a path".to_owned())?;
        root = Some(PathBuf::from(value));
    }
    Ok((root, rest))
}

/// The git top-level of `directory`, canonicalized; `Ok(None)` outside git,
/// `Err` when git itself fails (missing, dubious ownership, ...).
fn git_toplevel(directory: &Path) -> Result<Option<PathBuf>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| format!("git failed ({error})"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Ok(None);
        }
        return Err(format!("git failed ({})", stderr.trim()));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| "git failed (the top-level path is not UTF-8)".to_owned())?;
    Path::new(text.trim_end_matches(['\n', '\r']))
        .canonicalize()
        .map(Some)
        .map_err(|error| format!("git failed (cannot resolve its top-level: {error})"))
}

/// What to do when a binary would scan another tree.
const REMEDY: &str = "Build it from this checkout into a checkout-local target directory \
     (cargo run --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
     --target-dir target/arch-tools ...): under a CARGO_TARGET_DIR shared between checkouts \
     Cargo treats another checkout's build as fresh and does not rebuild it. Or pass \
     --root PATH to scan another tree on purpose";

/// Decide the scan root without touching process state.
///
/// An explicit root wins and must look like this repository. Otherwise the
/// compiled tree is used only when it is `cwd`'s git top-level.
pub fn resolve_scan_root(
    explicit: Option<&Path>,
    compiled: &Path,
    cwd: &Path,
) -> Result<PathBuf, String> {
    if let Some(explicit) = explicit {
        let root = explicit
            .canonicalize()
            .map_err(|error| format!("cannot resolve --root {}: {error}", explicit.display()))?;
        if !root.join("Cargo.toml").is_file() || !root.join("tools/architecture").is_dir() {
            return Err(format!(
                "--root {} is not a RustyCore checkout (no Cargo.toml and tools/architecture)",
                root.display()
            ));
        }
        return Ok(root);
    }
    let toplevel = git_toplevel(cwd).map_err(|error| {
        format!(
            "{error} in {}, so the scan root cannot be confirmed; this binary was compiled \
             from {}. {REMEDY}",
            cwd.display(),
            compiled.display()
        )
    })?;
    let Some(toplevel) = toplevel else {
        return Err(format!(
            "the current directory {} is not inside a git checkout, so the scan root cannot be \
             confirmed; this binary was compiled from {}. Run it from that checkout or pass \
             --root PATH",
            cwd.display(),
            compiled.display()
        ));
    };
    if toplevel != compiled {
        return Err(format!(
            "this binary was compiled from {} but the current checkout is {}; it would scan the \
             compiled tree and its verdict would not describe this checkout. {REMEDY}",
            compiled.display(),
            toplevel.display()
        ));
    }
    Ok(toplevel)
}

/// Resolve the scan root for this process, record it for every later scan
/// and announce it on stderr (stdout stays the tool's machine-readable output).
pub fn select_scan_root(tool: &str, explicit: Option<&Path>) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("cannot read the current directory: {error}"))?;
    let root = resolve_scan_root(explicit, &compiled_repository_root()?, &cwd)?;
    if SCAN_ROOT.set(root.clone()).is_err() && SCAN_ROOT.get() != Some(&root) {
        return Err("the scan root was already selected differently".to_owned());
    }
    eprintln!(
        "{tool}: scan root {}{}",
        root.display(),
        if explicit.is_some() {
            " (explicit --root)"
        } else {
            " (compiled tree = current git top-level)"
        }
    );
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "handler-contract-scan-root-{tag}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }

    fn git_init(path: &Path) {
        let status = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["init", "--quiet"])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn the_compiled_tree_is_accepted_from_its_own_checkout() {
        let compiled = compiled_repository_root().unwrap();
        assert_eq!(
            resolve_scan_root(None, &compiled, &compiled).unwrap(),
            compiled
        );
        // Any directory inside the checkout resolves to the same top-level.
        let nested = compiled.join("tools/architecture");
        assert_eq!(
            resolve_scan_root(None, &compiled, &nested).unwrap(),
            compiled
        );
    }

    #[test]
    fn another_checkout_is_refused_without_an_explicit_root() {
        let compiled = compiled_repository_root().unwrap();
        let other = scratch("other");
        git_init(&other);
        let error = resolve_scan_root(None, &compiled, &other).unwrap_err();
        assert!(error.contains("was compiled from"), "{error}");
        assert!(error.contains(&other.display().to_string()), "{error}");
        assert!(error.contains("--root"), "{error}");
        std::fs::remove_dir_all(other).unwrap();
    }

    #[test]
    fn a_directory_outside_git_is_refused() {
        let compiled = compiled_repository_root().unwrap();
        let outside = scratch("outside");
        // A temp dir may sit inside some unrelated repository on a dev host;
        // either refusal is correct, neither may pass.
        let error = resolve_scan_root(None, &compiled, &outside).unwrap_err();
        assert!(error.contains("--root"), "{error}");
        std::fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn a_git_failure_is_reported_as_such() {
        let compiled = compiled_repository_root().unwrap();
        // `git -C` into a missing directory fails without "not a git repository".
        let missing = scratch("git-failure").join("missing");
        let error = resolve_scan_root(None, &compiled, &missing).unwrap_err();
        assert!(error.starts_with("git failed ("), "{error}");
        assert!(error.contains("target/arch-tools"), "{error}");
        assert!(!error.contains("not inside a git checkout"), "{error}");
    }

    #[test]
    fn another_checkout_names_the_checkout_local_remedy() {
        let compiled = compiled_repository_root().unwrap();
        let other = scratch("remedy");
        git_init(&other);
        let error = resolve_scan_root(None, &compiled, &other).unwrap_err();
        assert!(error.contains("--target-dir target/arch-tools"), "{error}");
        assert!(error.contains("does not rebuild"), "{error}");
        std::fs::remove_dir_all(other).unwrap();
    }

    #[test]
    fn an_explicit_root_overrides_the_check_but_must_be_a_checkout() {
        let compiled = compiled_repository_root().unwrap();
        let elsewhere = scratch("explicit");
        git_init(&elsewhere);
        // Explicit: the compiled tree is scanned even from another checkout.
        assert_eq!(
            resolve_scan_root(Some(&compiled), &compiled, &elsewhere).unwrap(),
            compiled
        );
        let error = resolve_scan_root(Some(&elsewhere), &compiled, &compiled).unwrap_err();
        assert!(error.contains("is not a RustyCore checkout"), "{error}");
        let missing = elsewhere.join("missing");
        assert!(
            resolve_scan_root(Some(&missing), &compiled, &compiled)
                .unwrap_err()
                .contains("cannot resolve --root")
        );
        std::fs::remove_dir_all(elsewhere).unwrap();
    }

    #[test]
    fn the_root_option_is_taken_from_any_position() {
        let arguments = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
        let (root, rest) =
            take_root_argument(arguments(&["check", "--root", "/r", "--syntax-only"])).unwrap();
        assert_eq!(root, Some(PathBuf::from("/r")));
        assert_eq!(rest, arguments(&["check", "--syntax-only"]));
        let (root, rest) = take_root_argument(arguments(&["check"])).unwrap();
        assert_eq!(root, None);
        assert_eq!(rest, arguments(&["check"]));
        assert!(take_root_argument(arguments(&["check", "--root"])).is_err());
        assert!(take_root_argument(arguments(&["--root", "/a", "--root", "/b"])).is_err());
    }
}
