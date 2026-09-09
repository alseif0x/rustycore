//! Handler-contract regressions, part 2 of 3.
//!
//! Moved out of the tests.rs root under #660; every test is unchanged.

use super::*;

#[test]
fn registration_guard_rejects_registration_macro_exports_and_reexports() {
    let upstream_aliases =
        registration_alias_violations("pub use inventory::submit as hidden_upstream_submit;\n")
            .expect("upstream alias source parses");
    assert!(
        upstream_aliases
            .iter()
            .any(|violation| violation.contains("inventory registration macro")),
        "{upstream_aliases:?}"
    );

    let owner_reexport = analyze_inline_source(
        r#"
            macro_rules! register_move {
                ($opcode:ident) => {
                    inventory::submit! {
                        E { opcode: ClientOpcodes::$opcode }
                    }
                };
            }
            pub(crate) use register_move as hidden_register_move;
            register_move!(Alpha);
        "#,
    )
    .expect_err("an audited owner macro must not be reexported under an alias");
    assert!(
        owner_reexport.contains("aliases or reexports an audited handler registration macro"),
        "{owner_reexport}"
    );

    let exported_owner_macro = analyze_inline_source(
        r#"
            #[macro_export]
            macro_rules! register_move {
                ($opcode:ident) => {
                    inventory::submit! {
                        E { opcode: ClientOpcodes::$opcode }
                    }
                };
            }
            register_move!(Alpha);
        "#,
    )
    .expect_err("an audited owner macro must not use #[macro_export]");
    assert!(
        exported_owner_macro.contains("uses #[macro_export]"),
        "{exported_owner_macro}"
    );
}

#[test]
fn an_inner_attribute_split_by_a_comment_is_not_a_shebang() {
    let fixture = source_graph_fixture("splice-inner-attr");
    let crate_root = fixture.join("src/lib.rs");
    let child = fixture.join("src/child.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create crate root directory");
    fs::write(
        &crate_root,
        "#[cfg(test)]\n#[path = \"child.rs\"]\nmod child;\n",
    )
    .expect("write crate root");
    // `#!// keep` then `[cfg(test)]` on the next line is one inner attribute,
    // not a shebang. A first-line rule deleted the `#!//` and left a stray
    // `[cfg(test)]` behind, which does not parse.
    fs::write(&child, "#!// keep\n[cfg(test)]\npub fn thing() {}\n").expect("write child");

    let spliced = read_spliced_source(&crate_root, &fixture).expect("splice the path module");

    assert!(
        spliced.contains("pub fn thing()"),
        "the child's code must arrive: {spliced}"
    );
    syn::parse_file(&spliced).expect("the spliced source must still be valid Rust");

    fs::remove_dir_all(&fixture).expect("clean up fixture");
}

#[test]
fn a_child_shebang_does_not_survive_into_the_inline_module() {
    let fixture = source_graph_fixture("splice-shebang");
    let crate_root = fixture.join("src/lib.rs");
    let child = fixture.join("src/child.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create crate root directory");
    fs::write(&crate_root, "#[path = \"child.rs\"]\nmod child;\n").expect("write crate root");
    // rustc and rustfmt accept a shebang atop an external module file. Inside
    // `mod child { .. }` the same line is an inner attribute and does not
    // parse, so splicing it through made the whole parent unparseable and every
    // row it owned vanished from the inventory.
    fs::write(&child, "#!/usr/bin/env false\npub fn thing() {}\n").expect("write child");

    let spliced = read_spliced_source(&crate_root, &fixture).expect("splice the path module");

    assert!(
        !spliced.contains("#!/usr/bin/env"),
        "the shebang must not be carried inside the module: {spliced}"
    );
    assert!(
        spliced.contains("pub fn thing()"),
        "the child's code must still arrive: {spliced}"
    );
    syn::parse_file(&spliced).expect("the spliced source must still be valid Rust");

    fs::remove_dir_all(&fixture).expect("clean up fixture");
}

#[test]
fn path_module_is_spliced_back_into_its_parent_so_extraction_is_invisible() {
    let fixture = source_graph_fixture("splice-path");
    let crate_root = fixture.join("src/lib.rs");
    let child = fixture.join("src/thing_tests.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create crate root directory");
    fs::write(
        &crate_root,
        "use wow_entities::Creature;\n\n#[cfg(test)]\n#[path = \"thing_tests.rs\"]\nmod tests;\n",
    )
    .expect("write crate root");
    fs::write(
        &child,
        "//! Behaviour tests.\n#![cfg(test)]\n\nuse super::*;\n\nfn helper() -> Creature {\n    Creature::new(false)\n}\n",
    )
    .expect("write child");

    let spliced = read_spliced_source(&crate_root, &fixture).expect("splice the path module");

    // The parent's own text is untouched, so nothing outside the module moves.
    assert!(
        spliced.starts_with("use wow_entities::Creature;"),
        "the parent's own source must be preserved verbatim: {spliced}"
    );
    // The indirection is gone and the module is inline, which is what makes the
    // child inherit the parent's imports again instead of losing provenance
    // through `use super::*`.
    assert!(
        !spliced.contains("#[path"),
        "the #[path] attribute must not survive splicing: {spliced}"
    );
    assert!(
        spliced.contains("#[cfg(test)]"),
        "the module's own cfg must be preserved exactly as written: {spliced}"
    );
    assert!(
        spliced.contains("mod tests {"),
        "the module must become inline: {spliced}"
    );
    assert!(
        spliced.contains("Creature::new(false)"),
        "the child's body must be carried in: {spliced}"
    );
    // The duplicate inner cfg is dropped; the outer #[cfg(test)] already says it,
    // and recording it twice would change the module's exact cfg identity.
    assert!(
        !spliced.contains("#![cfg(test)]"),
        "the redundant inner cfg must be stripped: {spliced}"
    );
    // Every other inner attribute is carried through unchanged.
    assert!(
        spliced.contains("//! Behaviour tests."),
        "a module doc comment is part of the audited source: {spliced}"
    );
    syn::parse_file(&spliced).expect("the spliced source must still be valid Rust");

    fs::remove_dir_all(&fixture).expect("clean up fixture");
}

#[test]
fn splicing_a_path_module_without_the_matching_cfg_keeps_its_inner_attributes() {
    let fixture = source_graph_fixture("splice-no-cfg");
    let crate_root = fixture.join("src/lib.rs");
    let child = fixture.join("src/personal.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create crate root directory");
    fs::write(
        &crate_root,
        "#[path = \"personal.rs\"]\npub mod personal;\n",
    )
    .expect("write crate root");
    fs::write(&child, "//! Docs.\n#![allow(dead_code)]\n\nfn thing() {}\n").expect("write child");

    let spliced = read_spliced_source(&crate_root, &fixture).expect("splice the path module");

    // Without a `#[cfg(test)]` on the declaration there is nothing to deduplicate,
    // so inner attributes are carried in exactly as written rather than rewritten.
    assert!(
        spliced.contains("#![allow(dead_code)]"),
        "inner attributes unrelated to the module's cfg must survive: {spliced}"
    );
    assert!(
        spliced.contains("pub mod personal {"),
        "visibility must be preserved: {spliced}"
    );
    syn::parse_file(&spliced).expect("the spliced source must still be valid Rust");

    fs::remove_dir_all(&fixture).expect("clean up fixture");
}

#[test]
fn ownership_source_graph_follows_cfg_path_and_target_directories() {
    let fixture = source_graph_fixture("valid-path");
    let crate_root = fixture.join("src/lib.rs");
    let regular_path = fixture.join("src/nested/legitimate.rs");
    let target_path = fixture.join("target/generated.rs");
    fs::create_dir_all(regular_path.parent().expect("regular path parent"))
        .expect("create regular path directory");
    fs::create_dir_all(target_path.parent().expect("target path parent"))
        .expect("create target path directory");
    fs::write(
        &crate_root,
        r#"
            #[cfg(windows)]
            #[path = "nested/legitimate.rs"]
            mod legitimate;

            #[cfg(not(windows))]
            #[path = "../target/generated.rs"]
            mod generated;
        "#,
    )
    .expect("write crate root");
    fs::write(&regular_path, "pub fn legitimate() {}\n").expect("write regular path");
    fs::write(&target_path, "pub fn generated() {}\n").expect("write target path");

    let (sources, explicit_paths, _) =
        audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
            .expect("valid cfg-inactive #[path] modules");
    assert_eq!(explicit_paths, 2);
    assert!(sources.contains_key(&regular_path.canonicalize().unwrap()));
    assert!(sources.contains_key(&target_path.canonicalize().unwrap()));

    fs::remove_dir_all(&fixture).expect("remove valid path fixture");
}

#[test]
fn ownership_source_graph_rejects_path_escape_and_non_rust_extension() {
    for (name, declared_path, target_inside_package, expected_error) in [
        (
            "outside",
            "../../outside.rs",
            false,
            "resolves outside package root",
        ),
        (
            "extension",
            "nested/generated.inc",
            true,
            "must reference a .rs file",
        ),
    ] {
        let fixture = source_graph_fixture(name);
        let crate_root = fixture.join("package/src/lib.rs");
        let target = if target_inside_package {
            fixture.join("package/src/nested/generated.inc")
        } else {
            fixture.join("outside.rs")
        };
        fs::create_dir_all(crate_root.parent().expect("crate root parent"))
            .expect("create crate source");
        fs::create_dir_all(target.parent().expect("path target parent"))
            .expect("create path target parent");
        fs::write(
            &crate_root,
            format!("#[cfg(windows)]\n#[path = {declared_path:?}]\nmod escaped_or_non_rust;\n"),
        )
        .expect("write invalid path declaration");
        fs::write(&target, "pub fn hidden() {}\n").expect("write invalid path target");

        let error =
            audit_package_source_graph(&fixture.join("package"), std::slice::from_ref(&crate_root))
                .expect_err("invalid #[path] must fail closed");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );

        fs::remove_dir_all(&fixture).expect("remove invalid path fixture");
    }
}

#[test]
fn ownership_source_graph_rejects_conditional_and_inline_path_grammar() {
    for (name, source, expected_error) in [
        (
            "cfg-attr-path",
            r#"#[cfg_attr(windows, path = "hidden.rs")] mod hidden;"#,
            "module #[cfg_attr(..., path = ...)] is not allowed",
        ),
        (
            "path-on-inline",
            r#"#[path = "hidden.rs"] mod inline { pub fn visible() {} }"#,
            "inline module inline",
        ),
        (
            "path-inside-inline",
            r#"mod inline { #[path = "hidden.rs"] mod hidden; }"#,
            "declared inside an inline module",
        ),
    ] {
        let fixture = source_graph_fixture(name);
        let crate_root = fixture.join("src/lib.rs");
        fs::create_dir_all(crate_root.parent().expect("crate root parent"))
            .expect("create source directory");
        fs::write(&crate_root, source).expect("write invalid path grammar");

        let error = audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
            .expect_err("closed #[path] grammar must reject ambiguous resolution");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );

        fs::remove_dir_all(&fixture).expect("remove path grammar fixture");
    }
}

#[test]
fn ownership_is_logical_not_a_physical_handlers_prefix() {
    let fixture = source_graph_fixture("logical-owner");
    let crate_root = fixture.join("src/lib.rs");
    let shadow = fixture.join("src/handlers/shadow.rs");
    fs::create_dir_all(shadow.parent().expect("shadow parent"))
        .expect("create handler-looking dir");
    fs::write(&crate_root, r#"#[path = "handlers/shadow.rs"] mod shadow;"#)
        .expect("write physical-prefix mount");
    fs::write(
        &shadow,
        r#"
            inventory::submit! {
                E {
                    opcode: ClientOpcodes::Hidden,
                }
            }
        "#,
    )
    .expect("write hidden submission");

    let (sources, _, unconditional) =
        audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
            .expect("source graph resolves");
    let shadow = shadow.canonicalize().expect("canonical shadow");
    assert_eq!(
        sources.get(&shadow),
        Some(&BTreeSet::from(["crate::shadow".to_owned()]))
    );
    let error = audit_package_registration_sources("wow-world", &sources, &unconditional)
        .expect_err("a physical handlers prefix must not confer logical ownership");
    assert!(
        error.contains(
            "invokes inventory registration macro inventory::submit! outside the declared handler-registration owner"
        ),
        "{error}"
    );

    fs::remove_dir_all(&fixture).expect("remove logical ownership fixture");
}

#[test]
fn ownership_propagates_every_logical_remount_to_descendants() {
    let fixture = source_graph_fixture("logical-remount");
    let crate_root = fixture.join("src/lib.rs");
    let handlers_root = fixture.join("src/handlers/mod.rs");
    let child = fixture.join("src/handlers/child.rs");
    fs::create_dir_all(handlers_root.parent().expect("handlers parent"))
        .expect("create remount fixture");
    fs::write(
        &crate_root,
        r#"
            mod handlers;
            #[path = "handlers/mod.rs"]
            mod shadow;
        "#,
    )
    .expect("write dual mount");
    fs::write(&handlers_root, "mod child;\n").expect("write shared module root");
    fs::write(
        &child,
        r#"
            inventory::submit! {
                E {
                    opcode: ClientOpcodes::Hidden,
                }
            }
        "#,
    )
    .expect("write remounted child submission");

    let (sources, explicit_paths, unconditional) =
        audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
            .expect("every logical remount is traversed");
    assert_eq!(explicit_paths, 1);
    let child = child.canonicalize().expect("canonical child");
    assert_eq!(
        sources.get(&child),
        Some(&BTreeSet::from([
            "crate::handlers::child".to_owned(),
            "crate::shadow::child".to_owned(),
        ]))
    );
    let error = audit_package_registration_sources("wow-world", &sources, &unconditional)
        .expect_err("an outside remount must remove the handler-owner exemption from descendants");
    assert!(
        error.contains("inventory registration macro inventory::submit!"),
        "{error}"
    );

    fs::remove_dir_all(&fixture).expect("remove remount fixture");
}

#[test]
fn ownership_rejects_module_declarations_inside_item_bodies() {
    let fixture = source_graph_fixture("nested-module");
    let crate_root = fixture.join("src/lib.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create source directory");
    fs::write(
        &crate_root,
        r#"
            mod handlers {
                const INSTALL: () = {
                    mod hidden;
                };
            }
        "#,
    )
    .expect("write nested module declaration");

    let error = audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
        .expect_err("module declarations inside item bodies must fail closed");
    assert!(
        error.contains("declared inside a block/item body"),
        "{error}"
    );

    fs::remove_dir_all(&fixture).expect("remove nested module fixture");
}

#[test]
fn ownership_allows_only_the_exact_registry_module_collector() {
    let fixture = source_graph_fixture("collector-owner");
    let crate_root = fixture.join("src/lib.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create collector source directory");
    let canonical_root = {
        fs::write(&crate_root, "inventory::collect!(PacketHandlerEntry);\n")
            .expect("write exact collector");
        crate_root.canonicalize().expect("canonical collector root")
    };
    // #359 moved the collector out of wow-handler: the entry names WorldSession,
    // so it lives in the dispatcher owner's registry module.
    let sources = BTreeMap::from([(
        canonical_root.clone(),
        BTreeSet::from(["crate::session::registry".to_owned()]),
    )]);
    let unconditional: BTreeSet<_> = sources.keys().cloned().collect();

    audit_package_registration_sources("wow-world", &sources, &unconditional)
        .expect("one exact unconditional collector in the registry module must pass");
    let elsewhere = BTreeMap::from([(
        canonical_root,
        BTreeSet::from(["crate::session::driver".to_owned()]),
    )]);
    let error = audit_package_registration_sources("wow-world", &elsewhere, &unconditional)
        .expect_err("another session module must not own the collector");
    assert!(
        error.contains("inventory registration macro inventory::collect!"),
        "{error}"
    );

    for (name, source, expected_error) in [
        (
            "conditional",
            "#[cfg(windows)] inventory::collect!(PacketHandlerEntry);\n",
            "conditionally compiles inventory::collect!(PacketHandlerEntry)",
        ),
        (
            "duplicate",
            "inventory::collect!(PacketHandlerEntry);\n\
             inventory::collect!(PacketHandlerEntry);\n",
            "must define exactly one unconditional module-level",
        ),
        (
            "nested",
            "const INSTALL: () = { inventory::collect!(PacketHandlerEntry); };\n",
            "outside module item level",
        ),
        (
            "renamed-namespace",
            "use inv as inventory;\n\
             inventory::collect!(PacketHandlerEntry);\n",
            "can alias an inventory registration macro",
        ),
        (
            "raw-renamed-namespace",
            "extern crate inv as r#inventory;\n\
             r#inventory::collect!(PacketHandlerEntry);\n",
            "crate alias can hide inventory",
        ),
        (
            "module-namespace",
            "mod inventory { pub use inv::*; }\n\
             inventory::collect!(PacketHandlerEntry);\n",
            "shadows the canonical inventory crate namespace",
        ),
    ] {
        fs::write(&crate_root, source).expect("write collector mutant");
        let error = audit_package_registration_sources("wow-world", &sources, &unconditional)
            .expect_err("collector mutant must fail closed");
        assert!(
            error.contains(expected_error),
            "{name}: expected {expected_error:?}, got {error:?}"
        );
    }

    fs::write(&crate_root, "inventory::collect!(PacketHandlerEntry);\n")
        .expect("restore exact collector");
    let error = audit_package_registration_sources("world-server", &sources, &unconditional)
        .expect_err("the exact collector is forbidden outside the registry module");
    assert!(
        error.contains("inventory registration macro inventory::collect!"),
        "{error}"
    );

    fs::remove_dir_all(&fixture).expect("remove collector fixture");
}

#[test]
fn ownership_rejects_collector_mounted_below_a_conditional_parent() {
    for (name, parent_attribute) in [
        ("cfg", "#[cfg(windows)]"),
        (
            "cfg-attr",
            "#[cfg_attr(windows, cfg(target_pointer_width = \"16\"))]",
        ),
    ] {
        // Mount the collector at the owner path itself, so the only thing that
        // can disqualify it is the conditional parent (#363).
        let fixture = source_graph_fixture(name);
        let crate_root = fixture.join("src/lib.rs");
        let session_module = fixture.join("src/session/mod.rs");
        let collector_module = fixture.join("src/session/registry.rs");
        fs::create_dir_all(session_module.parent().expect("session module parent"))
            .expect("create collector fixture");
        fs::write(&crate_root, format!("{parent_attribute}\nmod session;\n"))
            .expect("write conditional collector parent");
        fs::write(&session_module, "pub mod registry;\n").expect("write session module");
        fs::write(
            &collector_module,
            "inventory::collect!(PacketHandlerEntry);\n",
        )
        .expect("write nested collector");

        let (sources, _, unconditional) =
            audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
                .expect("cfg-independent graph follows collector module");
        let error = audit_package_registration_sources("wow-world", &sources, &unconditional)
            .expect_err("a collector below a conditional parent does not own the collector");
        assert!(
            error.contains("inventory registration macro inventory::collect!"),
            "{name}: {error}"
        );

        fs::remove_dir_all(&fixture).expect("remove conditional collector fixture");
    }
}

#[test]
fn ownership_rejects_inventory_private_submit_aliases() {
    let error = reject_registration_syntax_outside_handlers(
        Path::new("crates/world-server/src/main.rs"),
        r#"
            fn hidden() {
                inventory::__do_submit! {
                    E {
                        opcode: ClientOpcodes::Hidden,
                    }
                }
            }
        "#,
    )
    .expect_err("inventory::__do_submit! must not bypass the owner");
    assert!(
        error.contains("inventory registration macro inventory::__do_submit!"),
        "{error}"
    );
}

#[test]
fn metadata_scope_uses_normal_workspace_package_ids_and_reverse_closure() {
    let package = |id: &str, name: &str| {
        json!({
            "id": id,
            "name": name,
            "manifest_path": format!("/repo/{name}/Cargo.toml"),
            "dependencies": [],
            "targets": []
        })
    };
    let dependency = |name: &str, package_id: &str, kind: Option<&str>, target: Option<&str>| {
        json!({
            "name": name,
            "pkg": package_id,
            "dep_kinds": [{"kind": kind, "target": target}]
        })
    };
    let metadata = json!({
        "packages": [
            package("handler", "wow-handler"),
            package("renamed-target", "renamed-target"),
            package("transitive", "transitive"),
            package("dev-only", "dev-only"),
            package("build-only", "build-only")
        ],
        "workspace_members": [
            "handler",
            "renamed-target",
            "transitive",
            "dev-only",
            "build-only"
        ],
        "resolve": {
            "nodes": [
                {"id": "handler", "deps": []},
                {
                    "id": "renamed-target",
                    "deps": [dependency(
                        "handler_alias",
                        "handler",
                        None,
                        Some("cfg(windows)")
                    )]
                },
                {
                    "id": "transitive",
                    "deps": [dependency("renamed-target", "renamed-target", None, None)]
                },
                {
                    "id": "dev-only",
                    "deps": [dependency("wow-handler", "handler", Some("dev"), None)]
                },
                {
                    "id": "build-only",
                    "deps": [dependency("wow-handler", "handler", Some("build"), None)]
                }
            ]
        }
    });

    assert_eq!(
        registry_capable_package_ids(&metadata).expect("valid synthetic metadata"),
        ["handler", "renamed-target", "transitive"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

#[test]
fn metadata_scope_rejects_non_workspace_reverse_dependencies() {
    let metadata = json!({
        "packages": [
            {
                "id": "handler",
                "name": "wow-handler",
                "manifest_path": "/repo/wow-handler/Cargo.toml"
            },
            {
                "id": "external-wrapper",
                "name": "external-wrapper",
                "manifest_path": "/cargo/git/external-wrapper/Cargo.toml"
            },
            {
                "id": "world",
                "name": "world-server",
                "manifest_path": "/repo/world-server/Cargo.toml"
            }
        ],
        "workspace_members": ["handler", "world"],
        "resolve": {
            "nodes": [
                {"id": "handler", "deps": []},
                {
                    "id": "external-wrapper",
                    "deps": [{
                        "name": "wow_handler",
                        "pkg": "handler",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }]
                },
                {
                    "id": "world",
                    "deps": [{
                        "name": "external_wrapper",
                        "pkg": "external-wrapper",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }]
                }
            ]
        }
    });

    let error = registry_capable_package_ids(&metadata)
        .expect_err("an external normal reverse dependency cannot be audited safely");
    assert!(
        error.contains("non-workspace package external-wrapper"),
        "{error}"
    );
}

#[test]
fn metadata_dependency_aliases_include_renamed_external_sqlx() {
    let metadata = json!({
        "packages": [
            {"id": "consumer", "name": "consumer"},
            {"id": "sqlx", "name": "sqlx"},
            {"id": "external", "name": "unrelated-external"}
        ],
        "workspace_members": ["consumer"],
        "resolve": {
            "nodes": [{
                "id": "consumer",
                "deps": [
                    {
                        "name": "db",
                        "pkg": "sqlx",
                        "dep_kinds": [{"kind": null, "target": null}]
                    },
                    {
                        "name": "helper",
                        "pkg": "external",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }
                ]
            }]
        }
    });

    let aliases = workspace_dependency_aliases_from_metadata(&metadata)
        .expect("valid synthetic dependency aliases");
    assert_eq!(
        aliases.production.get("consumer"),
        Some(&BTreeMap::from([("db".to_owned(), "sqlx".to_owned())]))
    );
    assert_eq!(aliases.test, aliases.production);
}

#[test]
fn metadata_scope_fails_closed_on_unknown_dependency_kinds() {
    for (name, dep_kinds, expected_error) in [
        (
            "missing",
            json!([{"target": null}]),
            "missing dep_kinds[].kind",
        ),
        (
            "unknown",
            json!([{"kind": "runtime", "target": null}]),
            "unsupported dependency kind",
        ),
        ("empty", json!([]), "has no dep_kinds entries"),
    ] {
        let metadata = json!({
            "packages": [
                {
                    "id": "handler",
                    "name": "wow-handler",
                    "manifest_path": "/repo/wow-handler/Cargo.toml"
                },
                {
                    "id": "consumer",
                    "name": "consumer",
                    "manifest_path": "/repo/consumer/Cargo.toml"
                }
            ],
            "workspace_members": ["handler", "consumer"],
            "resolve": {
                "nodes": [
                    {"id": "handler", "deps": []},
                    {
                        "id": "consumer",
                        "deps": [{
                            "name": "wow_handler",
                            "pkg": "handler",
                            "dep_kinds": dep_kinds
                        }]
                    }
                ]
            }
        });

        let error = registry_capable_package_ids(&metadata)
            .expect_err("unknown dependency kind must not disappear from the audit");
        assert!(
            error.contains(expected_error),
            "{name}: expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_discovers_direct_and_macro_generated_registrations() {
    let report = analyze_inline_source(
        r#"
            inventory::submit! {
                PacketHandlerEntry {
                    opcode: ClientOpcodes::Alpha,
                    status: SessionStatus::LoggedIn,
                    processing: PacketProcessing::Inplace,
                    handler_name: "alpha",
                }
            }

            macro_rules! register_handler {
                ($opcode:ident) => {
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                        }
                    }
                };
            }

            register_handler!(Beta);
        "#,
    )
    .expect("unconditional synthetic registrations must pass");

    assert_eq!(
        report,
        RegistrationSourceReport {
            direct_submissions: 1,
            registration_macro_invocations: 1,
            registration_macro_names: ["register_handler".to_owned()].into_iter().collect(),
        }
    );
}

#[test]
fn source_guard_recognizes_canonical_submit_without_a_type_spelling() {
    let report = analyze_inline_source(
        r#"
            inventory::submit! {
                E {
                    opcode: ClientOpcodes::Alpha,
                }
            }
        "#,
    )
    .expect("the canonical submit path is the owned registration grammar");
    assert_eq!(report.direct_submissions, 1);
}
