use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

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
}
