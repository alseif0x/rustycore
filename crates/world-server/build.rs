use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
#[cfg(feature = "forever-spell-random")]
#[path = "build/forever_spell_random.rs"]
mod forever_spell_random;
#[cfg(feature = "forever-spell-traversal")]
#[path = "build/forever_spell_traversal.rs"]
mod forever_spell_traversal;

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    Some(value.trim().to_owned())
}

fn valid_revision(revision: &str) -> bool {
    matches!(revision.len(), 40 | 64)
        && revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn resolve_revision(manifest_dir: &Path) -> String {
    env::var("GIT_HASH")
        .ok()
        .filter(|revision| valid_revision(revision))
        .or_else(|| {
            git_output(manifest_dir, &["rev-parse", "HEAD"])
                .filter(|revision| valid_revision(revision))
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn watch_git_identity(manifest_dir: &Path) {
    let Some(git_dir) = git_output(manifest_dir, &["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    println!("cargo:rerun-if-changed={git_dir}/HEAD");

    let Some(common_dir) = git_output(
        manifest_dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ) else {
        return;
    };
    println!("cargo:rerun-if-changed={common_dir}/packed-refs");

    let Some(symbolic_ref) = git_output(manifest_dir, &["symbolic-ref", "-q", "HEAD"]) else {
        return;
    };
    let ref_path = PathBuf::from(common_dir).join(symbolic_ref);
    println!("cargo:rerun-if-changed={}", ref_path.display());
}

fn main() {
    println!("cargo:rerun-if-env-changed=GIT_HASH");
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo always sets CARGO_MANIFEST_DIR"),
    );
    watch_git_identity(&manifest_dir);
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        resolve_revision(&manifest_dir)
    );
    #[cfg(feature = "forever-name-regex")]
    build_forever_names(&manifest_dir);
    #[cfg(feature = "forever-spell-traversal")]
    forever_spell_traversal::build(&manifest_dir);
    #[cfg(feature = "forever-spell-random")]
    forever_spell_random::build(&manifest_dir);
}

#[cfg(feature = "forever-name-regex")]
fn build_forever_names(manifest_dir: &Path) {
    const PIN: &str = "4cbcd3078e6ae10d05124379623a1bf03fcb9350";
    assert_eq!(
        env::var("CARGO_CFG_TARGET_OS").as_deref(),
        Ok("linux"),
        "Forever name engine currently requires the verified Linux host contract"
    );
    println!("cargo:rerun-if-env-changed=FOREVER_BOOST_REGEX_ROOT");
    let root = env::var_os("FOREVER_BOOST_REGEX_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../../target/forever-login/boost-regex-1.83"));
    assert_eq!(
        git_output(&root, &["rev-parse", "HEAD"]).as_deref(),
        Some(PIN),
        "Obtain the pinned Boost.Regex 1.83 checkout before enabling forever-name-regex; builds do not download dependencies"
    );
    assert_eq!(
        git_output(
            &root,
            &[
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--",
                "include"
            ]
        )
        .as_deref(),
        Some(""),
        "Boost.Regex headers must match the pinned clean source"
    );
    let bridge = manifest_dir.join("src/forever/name_regex/bridge.cpp");
    println!("cargo:rerun-if-changed={}", bridge.display());
    println!("cargo:rerun-if-changed={}", root.join("include").display());
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("BOOST_REGEX_STANDALONE", None)
        .include(root.join("include"))
        .file(bridge)
        .compile("rustycore_forever_name_regex");
}
