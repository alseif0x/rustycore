//! Compile the same small source tree that both audit walkers inspect.
//! Decoys are valid parseable Rust but fail compilation if rustc selects them.

use super::*;
use crate::ownership::audit_package_source_graph;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "rustycore-module-path-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root).unwrap();
        let fixture = Self(root);
        for (path, contents) in files {
            let path = fixture.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        fixture
    }

    fn verify(&self, expected: &[&str]) {
        self.verify_with_leaf_count(expected, 1);
    }

    fn verify_with_leaf_count(&self, expected: &[&str], leaf_count: usize) {
        let root = self.0.join("src/lib.rs");
        let compiler = Command::new("rustc")
            .args([
                "--edition=2024",
                "--crate-type=lib",
                "--emit=metadata",
                "-Awarnings",
            ])
            .arg(&root)
            .arg("-o")
            .arg(self.0.join("probe.rmeta"))
            .output()
            .expect("run the pinned Rust compiler for the module-path oracle");
        assert!(
            compiler.status.success(),
            "{}",
            String::from_utf8_lossy(&compiler.stderr)
        );

        let (sources, _, _) = audit_package_source_graph(&self.0, std::slice::from_ref(&root))
            .expect("audit must follow the source tree rustc just compiled");
        let expected: BTreeSet<_> = expected
            .iter()
            .map(|path| self.0.join(path).canonicalize().unwrap())
            .collect();
        assert_eq!(sources.keys().cloned().collect::<BTreeSet<_>>(), expected);

        let mut collection = SourceCollection::default();
        collect_source_file(
            &root,
            root.parent().unwrap(),
            "crate",
            None,
            &mut collection,
            &mut Vec::new(),
        )
        .expect("registration walker must follow the same tree");
        assert_eq!(
            collection
                .definitions
                .iter()
                .map(|definition| definition.name.as_str())
                .collect::<Vec<_>>(),
            vec!["selected_leaf"; leaf_count]
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only this test-created unique directory, never the workspace or temp root.
        let _ = fs::remove_dir_all(&self.0);
    }
}

const LEAF: &str = "macro_rules! selected_leaf { () => {} }";
const DECOY: &str = "compile_error!(\"wrong module search directory\");";

#[test]
fn explicit_file_mount_uses_its_parent_for_implicit_children() {
    Fixture::new(&[
        (
            "src/lib.rs",
            "#[path = \"mounted/renamed.rs\"] mod logical;",
        ),
        ("src/mounted/renamed.rs", "mod child;"),
        ("src/mounted/child.rs", LEAF),
        ("src/mounted/renamed/child.rs", DECOY),
    ])
    .verify(&[
        "src/lib.rs",
        "src/mounted/renamed.rs",
        "src/mounted/child.rs",
    ]);
}

#[test]
fn explicit_mod_rs_mount_keeps_its_parent_for_implicit_children() {
    Fixture::new(&[
        ("src/lib.rs", "#[path = \"mounted/mod.rs\"] mod logical;"),
        ("src/mounted/mod.rs", "mod child;"),
        ("src/mounted/child.rs", LEAF),
        ("src/logical/child.rs", DECOY),
    ])
    .verify(&["src/lib.rs", "src/mounted/mod.rs", "src/mounted/child.rs"]);
}

#[test]
fn explicit_file_mount_inline_and_ordinary_descendants_keep_distinct_directories() {
    Fixture::new(&[
        (
            "src/lib.rs",
            "#[path = \"mounted/renamed.rs\"] mod logical;",
        ),
        ("src/mounted/renamed.rs", "mod inline { mod child; }"),
        ("src/mounted/inline/child.rs", "mod deep;"),
        ("src/mounted/inline/child/deep.rs", LEAF),
        ("src/mounted/renamed/inline/child.rs", DECOY),
        ("src/mounted/inline/deep.rs", DECOY),
    ])
    .verify(&[
        "src/lib.rs",
        "src/mounted/renamed.rs",
        "src/mounted/inline/child.rs",
        "src/mounted/inline/child/deep.rs",
    ]);
}

#[test]
fn ordinary_file_mount_still_searches_children_under_its_stem() {
    Fixture::new(&[
        ("src/lib.rs", "mod ordinary;"),
        ("src/ordinary.rs", "mod child;"),
        ("src/ordinary/child.rs", LEAF),
        ("src/child.rs", DECOY),
    ])
    .verify(&["src/lib.rs", "src/ordinary.rs", "src/ordinary/child.rs"]);
}

#[test]
fn ordinary_and_explicit_mounts_of_one_file_traverse_their_own_children() {
    Fixture::new(&[
        (
            "src/lib.rs",
            "mod shared; #[path = \"shared.rs\"] mod alias;",
        ),
        ("src/shared.rs", "mod child;"),
        ("src/shared/child.rs", LEAF),
        ("src/child.rs", LEAF),
    ])
    .verify_with_leaf_count(
        &[
            "src/lib.rs",
            "src/shared.rs",
            "src/shared/child.rs",
            "src/child.rs",
        ],
        2,
    );
}
