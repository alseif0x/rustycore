//! Pinned, target-only container replay. No network or generated vendor files.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

// Official Boost 1.83.0 tar.bz2 SHA256:
// 6478edfe2f3305127cffe8caf73ea0176c53769f4bf1585be237eb30798c3b8e.
// Hash of sorted `sha256sum` lines for its boost/ tree + LICENSE_1_0.txt.
const HEADER_TREE: &str = "6442dc47d86b5c718d698d65a33ab098c13074a56ff2d7ca695689ffac41b711";

pub(super) fn build(manifest_dir: &Path) {
    for (variable, expected) in [
        ("CARGO_CFG_TARGET_OS", "linux"),
        ("CARGO_CFG_TARGET_ARCH", "x86_64"),
        ("CARGO_CFG_TARGET_ENV", "gnu"),
    ] {
        assert_eq!(
            std::env::var(variable).as_deref(),
            Ok(expected),
            "Forever spell traversal currently requires the pinned Linux x64 GNU contract"
        );
    }
    println!("cargo:rerun-if-env-changed=FOREVER_BOOST_HEADERS_ROOT");
    let root = std::env::var_os("FOREVER_BOOST_HEADERS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../../target/forever-login/boost_1_83_0"));
    assert_eq!(
        header_tree(&root).as_deref(),
        Some(HEADER_TREE),
        "Prepare the pinned Boost 1.83 header tree before enabling forever-spell-traversal; builds do not download or repair dependencies"
    );
    println!("cargo:rerun-if-changed={}", root.display());
    let directory = manifest_dir.join("src/forever/spell_traversal");
    println!("cargo:rerun-if-changed={}", directory.display());
    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .include(&root)
        .file(directory.join("bridge.cpp"))
        .file(directory.join("birth_lookup.cpp"))
        .file(directory.join("book_order.cpp"))
        .file(directory.join("override_order.cpp"))
        .compile("rustycore_forever_spell_traversal");
}

fn files(root: &Path, relative: &Path, output: &mut Vec<String>) -> Option<()> {
    let metadata = fs::symlink_metadata(root.join(relative)).ok()?;
    if metadata.is_symlink() {
        return None;
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(root.join(relative)).ok()? {
            let entry = entry.ok()?;
            files(root, &relative.join(entry.file_name()), output)?;
        }
    } else if metadata.is_file() {
        let name = relative.to_str()?;
        // Keep the GNU sha256sum text manifest unambiguous. Official paths
        // are ASCII and have no newline/backslash; do not accept escaped names.
        if !name.is_ascii()
            || name
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '\\'))
        {
            return None;
        }
        output.push(name.to_owned());
    } else {
        return None;
    }
    Some(())
}

fn header_tree(root: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(root).ok()?;
    if metadata.is_symlink() || !metadata.is_dir() {
        return None;
    }
    let mut paths = Vec::new();
    // Reject extra top-level inputs as well as missing/changed/extra headers.
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        if entry.file_name() != "boost" && entry.file_name() != "LICENSE_1_0.txt" {
            return None;
        }
    }
    files(root, Path::new("boost"), &mut paths)?;
    files(root, Path::new("LICENSE_1_0.txt"), &mut paths)?;
    paths.sort(); // ASCII byte order, matching LC_ALL=C sort during preparation
    let manifest = Command::new("sha256sum")
        .arg("--")
        .args(paths)
        .current_dir(root)
        .output()
        .ok()?;
    if !manifest.status.success() {
        return None;
    }
    let mut digest = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let write_ok = digest.stdin.take()?.write_all(&manifest.stdout).is_ok();
    let output = digest.wait_with_output().ok()?;
    if !write_ok || !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .split_whitespace()
        .next()
        .map(str::to_owned)
}
