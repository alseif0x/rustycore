// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::check_repository;
use crate::dispatcher::{
    DispatcherContract, KnownDispatchDrift, compare_dispatch_sides,
    dispatcher_contract_from_mounts, dispatcher_contract_from_source,
};
use crate::module_policy::{CapabilityOwner, parse_handler_module_policy};
use crate::ownership::{
    WorkspaceSourceMount, audit_package_registration_sources,
    audit_package_registration_sources_with_owner, audit_package_source_graph,
    audit_package_source_mounts, read_spliced_source, registry_capable_package_ids,
    workspace_dependency_aliases_from_metadata,
};
use crate::registrations::{
    RegistrationSourceReport, analyze_handler_mounts, analyze_inline_source, exported_macro_names,
    handler_capable_macro_definitions, handler_capable_macro_invocations, include_macro_bodies,
    inventory_registration_macro_fingerprints, registration_alias_violations,
    reject_registration_syntax_outside_handlers,
};
use crate::snapshot::{SnapshotContract, parse_snapshot_contract};
use serde_json::json;

#[test]
fn repository_handler_contract_passes() {
    let report = check_repository()
        .unwrap_or_else(|error| panic!("invalid repository handler contract:\n{error}"));
    assert!(report.starts_with("handler contract: PASS"), "{report}");
    assert!(report.contains("0 exact drift exceptions"), "{report}");
    assert!(report.contains("3 #[path] modules verified"), "{report}");
}

#[test]
fn dispatcher_parser_uses_grouped_top_level_patterns_not_body_mentions() {
    let source = r#"
        impl WorldSession {
            async fn dispatch_packet(&mut self) {
                let _other = match something_else {
                    _ => ClientOpcodes::NotTheDispatcher,
                };
                match opcode {
                    (ClientOpcodes::Alpha | ClientOpcodes::Beta) => {
                        let _ = ClientOpcodes::BodyOnly;
                    }
                    _ => {}
                }
            }
        }
    "#;

    let contract = dispatcher_contract_from_source(source).expect("synthetic dispatcher parses");
    assert_eq!(
        contract,
        DispatcherContract {
            opcode_names: ["Alpha".to_owned(), "Beta".to_owned()]
                .into_iter()
                .collect(),
        }
    );
}

#[test]
fn dispatcher_parser_rejects_unreachable_conditional_guarded_and_duplicate_arms() {
    let cases = [
        (
            r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        match opcode {
                            _ => {}
                            ClientOpcodes::Alpha => {}
                        }
                    }
                }
            "#,
            "wildcard arm must be last",
        ),
        (
            r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        match opcode {
                            ClientOpcodes::Alpha | _ => {}
                        }
                    }
                }
            "#,
            "wildcard must be a standalone `_` arm",
        ),
        (
            r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        match opcode {
                            #[cfg(test)]
                            ClientOpcodes::Alpha => {}
                            _ => {}
                        }
                    }
                }
            "#,
            "opcode arms must not be conditionally compiled",
        ),
        (
            r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        match opcode {
                            ClientOpcodes::Alpha if enabled => {}
                            _ => {}
                        }
                    }
                }
            "#,
            "opcode arms must not use match guards",
        ),
        (
            r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        match opcode {
                            ClientOpcodes::Alpha | ClientOpcodes::Alpha => {}
                            _ => {}
                        }
                    }
                }
            "#,
            "duplicate opcode arm Alpha",
        ),
    ];

    for (source, expected_error) in cases {
        let error =
            dispatcher_contract_from_source(source).expect_err("unsafe dispatcher shape must fail");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

fn dispatcher_owner() -> CapabilityOwner {
    CapabilityOwner {
        capability: "packet_dispatcher".to_owned(),
        package: "wow-world".to_owned(),
        module: "crate::session".to_owned(),
        allow_descendants: true,
        tracking_issue: 152,
    }
}

fn dispatcher_body(opcode: &str) -> String {
    format!(
        r#"
            impl crate::session::WorldSession {{
                async fn dispatch_packet(&mut self) {{
                    match opcode {{
                        ClientOpcodes::{opcode} => {{}},
                        _ => {{}},
                    }}
                }}
            }}
        "#
    )
}

fn fixture_workspace_mounts(
    package: &str,
    package_root: &Path,
    crate_root: &Path,
) -> Vec<WorkspaceSourceMount> {
    let (mounts, _) = audit_package_source_mounts(package_root, &[crate_root.to_owned()])
        .expect("fixture module graph resolves");
    mounts
        .into_iter()
        .map(|(source_path, contexts)| WorkspaceSourceMount {
            package: package.to_owned(),
            source: fs::read_to_string(&source_path).expect("read fixture source"),
            source_path,
            contexts,
        })
        .collect()
}

#[test]
fn module_aware_dispatcher_follows_a_private_child_independent_of_filename() {
    let fixture = source_graph_fixture("dispatcher-private-child");
    let crate_root = fixture.join("src/lib.rs");
    let session = fixture.join("src/session.rs");
    let private_child = fixture.join("src/session/router.rs");
    fs::create_dir_all(private_child.parent().expect("private child parent"))
        .expect("create dispatcher fixture");
    fs::write(&crate_root, "mod session;\n").expect("write crate root");
    fs::write(&session, "pub struct WorldSession;\nmod router;\n").expect("write session module");
    fs::write(
        &private_child,
        dispatcher_body("Alpha").replace(
            "impl crate::session::WorldSession",
            "#[cfg_attr(feature = \"lint-only\", allow(dead_code))]\nimpl crate::session::WorldSession",
        ),
    )
    .expect("write private dispatcher");

    let mounts = fixture_workspace_mounts("wow-world", &fixture, &crate_root);
    let contract = dispatcher_contract_from_mounts(&mounts, &dispatcher_owner())
        .expect("private child dispatcher remains valid");
    assert_eq!(contract.opcode_names, BTreeSet::from(["Alpha".to_owned()]));

    fs::remove_dir_all(&fixture).expect("remove dispatcher fixture");

    let inline_fixture = source_graph_fixture("dispatcher-inline-child");
    let inline_root = inline_fixture.join("src/lib.rs");
    let inline_session = inline_fixture.join("src/session.rs");
    fs::create_dir_all(inline_root.parent().expect("inline crate root parent"))
        .expect("create inline dispatcher fixture");
    fs::write(&inline_root, "mod session;\n").expect("write inline crate root");
    fs::write(
        &inline_session,
        format!(
            "pub struct WorldSession;\nmod private_router {{ {} }}\n",
            dispatcher_body("Beta")
        ),
    )
    .expect("write inline private dispatcher");
    let mounts = fixture_workspace_mounts("wow-world", &inline_fixture, &inline_root);
    let contract = dispatcher_contract_from_mounts(&mounts, &dispatcher_owner())
        .expect("inline private child dispatcher remains valid");
    assert_eq!(contract.opcode_names, BTreeSet::from(["Beta".to_owned()]));
    fs::remove_dir_all(&inline_fixture).expect("remove inline dispatcher fixture");

    let path_fixture = source_graph_fixture("dispatcher-path-child");
    let path_root = path_fixture.join("src/lib.rs");
    let path_session = path_fixture.join("src/session.rs");
    let path_child = path_fixture.join("src/session/nonstandard-name.rs");
    fs::create_dir_all(path_child.parent().expect("path child parent"))
        .expect("create path dispatcher fixture");
    fs::write(&path_root, "mod session;\n").expect("write path crate root");
    fs::write(
        &path_session,
        "pub struct WorldSession;\n#[path = \"session/nonstandard-name.rs\"] mod router;\n",
    )
    .expect("write path session module");
    fs::write(&path_child, dispatcher_body("Gamma")).expect("write path dispatcher");
    let mounts = fixture_workspace_mounts("wow-world", &path_fixture, &path_root);
    let contract = dispatcher_contract_from_mounts(&mounts, &dispatcher_owner())
        .expect("supported path dispatcher remains valid");
    assert_eq!(contract.opcode_names, BTreeSet::from(["Gamma".to_owned()]));
    fs::remove_dir_all(&path_fixture).expect("remove path dispatcher fixture");
}

#[test]
fn module_aware_dispatcher_rejects_missing_duplicate_conditional_and_outside_owners() {
    for (name, crate_source, files, expected_error) in [
        (
            "missing",
            "mod session;\n",
            vec![("src/session.rs", "pub struct WorldSession;\n".to_owned())],
            "found 0",
        ),
        (
            "duplicate",
            "mod session;\n",
            vec![
                (
                    "src/session.rs",
                    "pub struct WorldSession; mod first; mod second;\n".to_owned(),
                ),
                ("src/session/first.rs", dispatcher_body("Alpha")),
                ("src/session/second.rs", dispatcher_body("Beta")),
            ],
            "found 2",
        ),
        (
            "conditional",
            "mod session;\n",
            vec![
                (
                    "src/session.rs",
                    "pub struct WorldSession;\n#[cfg(feature = \"conditional-dispatch\")] mod router;\n".to_owned(),
                ),
                ("src/session/router.rs", dispatcher_body("Alpha")),
            ],
            "conditional module/impl/method ownership",
        ),
        (
            "outside-owner",
            "mod session;\n#[path = \"session/router.rs\"] mod shadow;\n",
            vec![
                ("src/session.rs", "pub struct WorldSession;\n".to_owned()),
                ("src/session/router.rs", dispatcher_body("Alpha")),
            ],
            "outside declared capability owner",
        ),
        (
            "homonym",
            "mod session;\n",
            vec![
                (
                    "src/session.rs",
                    "pub struct WorldSession; mod fake;\n".to_owned(),
                ),
                (
                    "src/session/fake.rs",
                    dispatcher_body("Alpha")
                        .replace("crate::session::WorldSession", "WorldSession")
                        .replacen("impl WorldSession", "struct WorldSession; impl WorldSession", 1),
                ),
            ],
            "does not implement the canonical",
        ),
        (
            "remount",
            "mod session;\n",
            vec![
                (
                    "src/session.rs",
                    "pub struct WorldSession;\n\
                     #[path = \"session/shared.rs\"] mod first;\n\
                     #[path = \"session/shared.rs\"] mod second;\n"
                        .to_owned(),
                ),
                ("src/session/shared.rs", dispatcher_body("Alpha")),
            ],
            "found 2",
        ),
    ] {
        let fixture = source_graph_fixture(name);
        let crate_root = fixture.join("src/lib.rs");
        fs::create_dir_all(crate_root.parent().expect("crate root parent"))
            .expect("create dispatcher rejection fixture");
        fs::write(&crate_root, crate_source).expect("write crate root");
        for (relative_path, source) in files {
            let path = fixture.join(relative_path);
            fs::create_dir_all(path.parent().expect("fixture source parent"))
                .expect("create fixture source parent");
            fs::write(path, source).expect("write fixture source");
        }
        let mounts = fixture_workspace_mounts("wow-world", &fixture, &crate_root);
        let error = dispatcher_contract_from_mounts(&mounts, &dispatcher_owner())
            .expect_err("invalid module-aware dispatcher ownership must fail");
        assert!(
            error.contains(expected_error),
            "{name}: expected {expected_error:?}, got {error:?}"
        );
        fs::remove_dir_all(&fixture).expect("remove dispatcher rejection fixture");
    }
}

#[test]
fn module_aware_registration_scan_uses_logical_mounts_and_rejects_duplicate_owners() {
    let owner = CapabilityOwner {
        capability: "handler_registration".to_owned(),
        package: "wow-world".to_owned(),
        module: "crate::handlers".to_owned(),
        allow_descendants: true,
        tracking_issue: 153,
    };
    let fixture = source_graph_fixture("registration-logical-mounts");
    let crate_root = fixture.join("src/lib.rs");
    let handlers = fixture.join("src/handlers.rs");
    let child = fixture.join("src/handlers/child.rs");
    fs::create_dir_all(child.parent().expect("registration child parent"))
        .expect("create registration fixture");
    fs::write(&crate_root, "mod handlers;\n").expect("write registration crate root");
    fs::write(&handlers, "mod child;\n").expect("write registration owner root");
    fs::write(
        &child,
        "inventory::submit! { PacketHandlerEntry { opcode: ClientOpcodes::Alpha } }\n",
    )
    .expect("write child registration");
    let mounts = fixture_workspace_mounts("wow-world", &fixture, &crate_root);
    let report = analyze_handler_mounts(&mounts, &owner)
        .expect("registration scanner follows the logical owner mounts");
    assert_eq!(report.direct_submissions, 1);
    fs::remove_dir_all(&fixture).expect("remove registration fixture");

    let duplicate_fixture = source_graph_fixture("registration-duplicate-owner");
    let duplicate_root = duplicate_fixture.join("src/lib.rs");
    let duplicate_handlers = duplicate_fixture.join("src/handlers.rs");
    let shared = duplicate_fixture.join("src/handlers/shared.rs");
    fs::create_dir_all(shared.parent().expect("shared registration parent"))
        .expect("create duplicate registration fixture");
    fs::write(&duplicate_root, "mod handlers;\n").expect("write duplicate crate root");
    fs::write(
        &duplicate_handlers,
        "#[path = \"handlers/shared.rs\"] mod first;\n\
         #[path = \"handlers/shared.rs\"] mod second;\n",
    )
    .expect("write duplicate logical mounts");
    fs::write(&shared, "pub fn harmless() {}\n").expect("write shared registration source");
    let mounts = fixture_workspace_mounts("wow-world", &duplicate_fixture, &duplicate_root);
    let error = analyze_handler_mounts(&mounts, &owner)
        .expect_err("a source mounted under two capability owners must fail");
    assert!(
        error.contains("duplicate or mixed logical ownership"),
        "{error}"
    );
    fs::remove_dir_all(&duplicate_fixture).expect("remove duplicate registration fixture");
}

#[test]
fn handler_module_policy_is_strict_and_registration_uses_declared_owner() {
    let valid = r#"{
        "schema_version": 1,
        "introduced_by_issue": 185,
        "capability_owners": [
            {"capability":"handler_registration","package":"wow-world","module":"crate::installers","allow_descendants":true,"tracking_issue":153},
            {"capability":"packet_dispatcher","package":"wow-world","module":"crate::session","allow_descendants":true,"tracking_issue":152}
        ]
    }"#;
    let policy = parse_handler_module_policy(valid).expect("valid module policy");
    let owner = policy.owner("handler_registration");

    let fixture = source_graph_fixture("declared-registration-owner");
    let outside = fixture.join("outside.rs");
    fs::create_dir_all(&fixture).expect("create registration owner fixture");
    fs::write(
        &outside,
        "inventory::submit! { E { opcode: ClientOpcodes::Hidden } }\n",
    )
    .expect("write outside registration");
    let sources = BTreeMap::from([(
        outside.canonicalize().expect("canonical outside source"),
        BTreeSet::from(["crate::handlers".to_owned()]),
    )]);
    let error = audit_package_registration_sources_with_owner(
        "wow-world",
        &sources,
        &BTreeSet::new(),
        owner,
    )
    .expect_err("registration outside declared policy owner must fail");
    assert!(error.contains("inventory registration macro"), "{error}");
    fs::remove_dir_all(&fixture).expect("remove registration owner fixture");

    for (source, expected_error) in [
        (
            valid.replace("\"schema_version\": 1", "\"schema_version\": 2"),
            "schema_version must be 1",
        ),
        (
            valid.replace("\"tracking_issue\":152", "\"tracking_issue\":0"),
            "has no tracking issue",
        ),
        (
            valid.replace("crate::session", "session"),
            "invalid logical module",
        ),
        (
            valid.replace(
                "\"capability\":\"packet_dispatcher\"",
                "\"capability\":\"handler_registration\"",
            ),
            "duplicate capability",
        ),
        (
            valid.replace(
                "\"schema_version\": 1,",
                "\"schema_version\": 1, \"unknown\": true,",
            ),
            "unknown field",
        ),
        (
            valid.replace("crate::installers", "crate::session::installers"),
            "overlapping logical owners",
        ),
    ] {
        let error = parse_handler_module_policy(&source).expect_err("malformed policy must fail");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn dispatch_comparison_rejects_new_and_obsolete_mismatches() {
    fn names(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    let registered = names(&["Alpha", "RegisteredOnly"]);
    let dispatched = names(&["Alpha", "DispatchedOnly"]);
    let tracked_registered_only = [KnownDispatchDrift {
        opcode_name: "RegisteredOnly",
        tracking_issue: 142,
    }];
    let tracked_dispatched_only = [KnownDispatchDrift {
        opcode_name: "DispatchedOnly",
        tracking_issue: 142,
    }];
    assert!(
        compare_dispatch_sides(
            &registered,
            &dispatched,
            &tracked_registered_only,
            &tracked_dispatched_only,
        )
        .is_ok(),
        "the exact tracked baseline must pass"
    );

    let error = compare_dispatch_sides(&registered, &dispatched, &[], &[])
        .expect_err("new mismatches must fail");
    assert!(
        error.contains(
            "registered opcode RegisteredOnly has no dispatcher arm and no tracked exception"
        ),
        "{error}"
    );
    assert!(
        error
            .contains("dispatcher arm DispatchedOnly has no registration and no tracked exception"),
        "{error}"
    );

    let corrected = names(&["Alpha", "RegisteredOnly"]);
    let error = compare_dispatch_sides(
        &registered,
        &corrected,
        &tracked_registered_only,
        &tracked_dispatched_only,
    )
    .expect_err("stale mismatch exceptions must fail");
    assert!(
        error.contains("obsolete registered-without-arm exception RegisteredOnly tracked by #142"),
        "{error}"
    );
    assert!(
        error
            .contains("obsolete arm-without-registration exception DispatchedOnly tracked by #142"),
        "{error}"
    );
}

#[test]
fn snapshot_parser_requires_exact_five_column_unique_rows() {
    let valid = "\
opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing\n\
0x0001\tAlpha\thandle_alpha\tLoggedIn\tInplace\n";
    assert_eq!(
        parse_snapshot_contract(valid).expect("valid contract"),
        SnapshotContract {
            row_count: 1,
            opcode_names: ["Alpha".to_owned()].into_iter().collect(),
        }
    );

    for (snapshot, expected_error) in [
        (
            "wrong\theader\n0x0001\tAlpha\th\tLoggedIn\tInplace\n",
            "expected",
        ),
        (
            "opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing\n\
             0x0001\tAlpha\th\tLoggedIn\n",
            "expected 5",
        ),
        (
            "opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing\n\
             0x0001\tAlpha\th\tLoggedIn\tInplace\n\
             0x0002\tAlpha\th2\tLoggedIn\tInplace\n",
            "duplicates opcode name Alpha",
        ),
    ] {
        let error = parse_snapshot_contract(snapshot).expect_err("invalid snapshot must fail");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn ownership_guard_rejects_cfg_inactive_session_registration() {
    let session_path = Path::new("crates/wow-world/src/session.rs");
    let error = reject_registration_syntax_outside_handlers(
        session_path,
        r#"
            #[cfg(windows)]
            inventory::submit! {
                PacketHandlerEntry {
                    opcode: ClientOpcodes::Alpha,
                    status: SessionStatus::LoggedIn,
                    processing: PacketProcessing::Inplace,
                    handler_name: "alpha",
                }
            }
        "#,
    )
    .expect_err("a target-inactive handler registration outside handlers must fail");
    assert!(
        error.contains(
            "session.rs invokes inventory registration macro inventory::submit! outside the \
             declared handler-registration owner"
        ),
        "{error}"
    );

    let alias_error = reject_registration_syntax_outside_handlers(
        session_path,
        r#"
            #[cfg(windows)]
            submit_alias! {
                PacketHandlerEntry {
                    opcode: ClientOpcodes::Alpha,
                    status: SessionStatus::LoggedIn,
                    processing: PacketProcessing::Inplace,
                    handler_name: "alpha",
                }
            }
        "#,
    )
    .expect_err("a submission alias mentioning PacketHandlerEntry must fail");
    assert!(
        alias_error.contains(
            "macro call mentioning PacketHandlerEntry outside the declared handler-registration owner"
        ),
        "{alias_error}"
    );

    let macro_error = reject_registration_syntax_outside_handlers(
        session_path,
        r#"
            #[cfg(windows)]
            register_move!(MoveStartForward);
        "#,
    )
    .expect_err("a target-inactive audited registration macro outside handlers must fail");
    assert!(
        macro_error.contains(
            "session.rs invokes audited handler registration macro register_move! outside the \
             declared handler-registration owner"
        ),
        "{macro_error}"
    );

    let include_error = reject_registration_syntax_outside_handlers(
        session_path,
        r#"
            #[cfg(windows)]
            include!(concat!(env!("OUT_DIR"), "/generated_handlers.rs"));
        "#,
    )
    .expect_err("include! outside handlers must fail even when target-inactive");
    assert!(
        include_error
            .contains("session.rs uses include! outside the declared handler-registration owner"),
        "{include_error}"
    );

    let other_registry_error = reject_registration_syntax_outside_handlers(
        session_path,
        r#"
            #[cfg(test)]
            inventory::submit! {
                wow_script::player::GivePlayerXpHookLikeCpp {
                    name: "not_a_packet_handler",
                    callback: callback,
                }
            }
        "#,
    )
    .expect_err("all production inventory::submit! calls outside the owner must fail closed");
    assert!(
        other_registry_error.contains("invokes inventory registration macro inventory::submit!"),
        "{other_registry_error}"
    );

    for (source, expected_error) in [
        (
            r#"
                use inv::submit as s;
                fn hidden() { s! { E { opcode: ClientOpcodes::Hidden } } }
            "#,
            "import use inv :: submit as s",
        ),
        (
            r#"
                #[macro_use]
                extern crate inventory as inv;
                fn hidden() { submit! { E { opcode: ClientOpcodes::Hidden } } }
            "#,
            "extern crate inventory as inv",
        ),
    ] {
        let error = reject_registration_syntax_outside_handlers(session_path, source)
            .expect_err("submit imports and aliases outside the owner must fail closed");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn registration_guard_rejects_inventory_namespace_spoofs() {
    let cases = [
        r#"
            use inv as inventory;
            inventory::submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            extern crate inv as inventory;
            inventory::submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            use evil::inventory;
            inventory::submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            use crate::{inventory};
            inventory::submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            mod inventory { pub use inv::*; }
            inventory::submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            #[path = "fake_inventory.rs"]
            mod inventory;
            inventory::submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            use inv as r#inventory;
            r#inventory::r#submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
        r#"
            extern crate inv as r#inventory;
            r#inventory::r#submit! { E { opcode: ClientOpcodes::Hidden } }
        "#,
    ];

    for source in cases {
        let outside_error = reject_registration_syntax_outside_handlers(
            Path::new("crates/world-server/src/main.rs"),
            source,
        )
        .expect_err("an inventory namespace spoof outside the owner must fail");
        assert!(
            outside_error.contains("inventory")
                && (outside_error.contains("alias")
                    || outside_error.contains("shadows")
                    || outside_error.contains("crate")),
            "{outside_error}"
        );

        let handler_error = analyze_inline_source(source)
            .expect_err("the handler owner must reject the same spoof");
        assert!(
            handler_error.contains("inventory")
                && (handler_error.contains("alias")
                    || handler_error.contains("shadows")
                    || handler_error.contains("crate")),
            "{handler_error}"
        );
    }

    let harmless_import = "use inventory::unrelated_symbol;\n";
    reject_registration_syntax_outside_handlers(
        Path::new("crates/world-server/src/main.rs"),
        harmless_import,
    )
    .expect("importing a non-registration symbol does not shadow inventory");
    analyze_inline_source(harmless_import)
        .expect("handler source may import a non-registration inventory symbol");
}

#[test]
fn registration_guard_handles_absolute_and_raw_macro_paths() {
    for macro_path in [
        "::inventory::submit",
        "::inv::submit",
        "r#inventory::r#submit",
    ] {
        let source =
            format!("#[cfg(windows)] {macro_path}! {{ E {{ opcode: ClientOpcodes::Hidden }} }}");
        let error = reject_registration_syntax_outside_handlers(
            Path::new("crates/wow-world/src/session.rs"),
            &source,
        )
        .expect_err("absolute/raw submit path outside the owner must not disappear");
        assert!(
            error.contains("inventory registration macro") && error.contains("submit!"),
            "{macro_path}: {error}"
        );
    }

    for macro_path in ["::inventory::submit", "r#inventory::r#submit"] {
        let report = analyze_inline_source(&format!(
            "{macro_path}! {{ E {{ opcode: ClientOpcodes::Alpha }} }}"
        ))
        .expect("canonical absolute/raw inventory paths remain auditable");
        assert_eq!(report.direct_submissions, 1, "{macro_path}");
    }
}

#[test]
fn registration_guard_rejects_metavariable_macro_forwarders() {
    let definition = r#"
        macro_rules! forward {
            ($registration:path, $entry:expr) => {
                $registration! { $entry }
            };
        }
    "#;
    let definition_error = reject_registration_syntax_outside_handlers(
        Path::new("crates/wow-world/src/session.rs"),
        definition,
    )
    .expect_err("a macro-metavariable invocation can forward an unowned registration");
    assert!(
        definition_error.contains("handler-capable macro_rules! forward"),
        "{definition_error}"
    );

    let invocation = r#"
        type Hidden = PacketHandlerEntry;
        #[cfg(windows)]
        external_forward!(
            inventory::submit,
            Hidden {
                opcode: ClientOpcodes::Hidden,
            }
        );
    "#;
    let invocation_error = reject_registration_syntax_outside_handlers(
        Path::new("crates/wow-world/src/session.rs"),
        invocation,
    )
    .expect_err("passing inventory::submit through an unknown macro must fail source audit");
    assert!(
        invocation_error.contains("passes an inventory registration path")
            && invocation_error.contains("external_forward"),
        "{invocation_error}"
    );

    let mount_error = reject_registration_syntax_outside_handlers(
        Path::new("crates/wow-world/src/session.rs"),
        "mount_source! { mod hidden_module; }\n",
    )
    .expect_err("an unknown macro must not mount an unaudited Rust module");
    assert!(
        mount_error.contains("passes handler-capable source tokens")
            && mount_error.contains("mount_source"),
        "{mount_error}"
    );
}

#[test]
fn registration_guard_rejects_workspace_macro_generators_before_export() {
    let definitions = handler_capable_macro_definitions(
        Path::new("crates/upstream/src/lib.rs"),
        r#"
            #[macro_export]
            macro_rules! hidden_submit {
                ($entry:expr) => {
                    inventory::submit! { $entry }
                };
            }
        "#,
    )
    .expect("workspace macro source parses");
    assert_eq!(definitions, ["hidden_submit"]);

    let harmless = handler_capable_macro_definitions(
        Path::new("crates/upstream/src/lib.rs"),
        r#"
            #[macro_export]
            macro_rules! forward_log {
                ($($arg:tt)*) => {
                    tracing::info!($($arg)*)
                };
            }
        "#,
    )
    .expect("ordinary exported expression macro parses");
    assert!(harmless.is_empty(), "{harmless:?}");

    let grouped_alias = handler_capable_macro_definitions(
        Path::new("crates/upstream/src/lib.rs"),
        r#"
            #[macro_export]
            macro_rules! hidden_grouped_submit {
                ($entry:expr) => {{
                    use inv::{submit as hidden_submit};
                    hidden_submit! { $entry }
                }};
            }

            macro_rules! hidden_grouped_collect {
                ($entry:ty) => {
                    use inv::{collect as hidden_collect};
                    hidden_collect! { $entry }
                };
            }
        "#,
    )
    .expect("grouped inventory alias macro source parses");
    assert_eq!(
        grouped_alias,
        ["hidden_grouped_collect", "hidden_grouped_submit"]
    );

    let meta_generator = handler_capable_macro_definitions(
        Path::new("crates/upstream/src/lib.rs"),
        r#"
            macro_rules! define_hidden {
                ($name:ident, $body:tt) => {
                    #[macro_export]
                    macro_rules! $name $body
                };
            }
        "#,
    )
    .expect("meta-macro source parses");
    assert_eq!(meta_generator, ["define_hidden"]);

    let meta_invocations = handler_capable_macro_invocations(
        Path::new("crates/upstream/src/lib.rs"),
        r#"
            define_hidden!(
                hidden,
                { ($entry:expr) => { inventory::submit! { $entry } }; }
            );
            mount_source! { mod hidden_module; }
        "#,
    )
    .expect("source-generating invocations parse");
    assert!(
        meta_invocations.iter().any(|name| name == "define_hidden")
            && meta_invocations.iter().any(|name| name == "mount_source"),
        "{meta_invocations:?}"
    );

    let includes = include_macro_bodies(
        Path::new("crates/upstream/src/lib.rs"),
        r#"include!("hidden.rs");"#,
    )
    .expect("literal include parses");
    assert_eq!(includes, [r#""hidden.rs""#]);

    let exports = exported_macro_names(
        Path::new("crates/upstream/src/lib.rs"),
        r#"
            #[macro_export]
            macro_rules! hidden_export { () => {}; }
        "#,
    )
    .expect("exported macro source parses");
    assert_eq!(exports, ["hidden_export"]);

    let inventory_calls = inventory_registration_macro_fingerprints(
        Path::new("crates/upstream/src/lib.rs"),
        "inventory::submit! { Hidden { opcode: ClientOpcodes::Hidden } }\n",
    )
    .expect("inventory macro source parses");
    assert_eq!(inventory_calls.len(), 1);
    assert!(inventory_calls[0].starts_with("inventory::submit!{"));
}

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

fn source_graph_fixture(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "handler-contract-check-{name}-{}-{unique}",
        std::process::id()
    ))
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

    let (sources, explicit_paths) =
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

    let (sources, _) = audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
        .expect("source graph resolves");
    let shadow = shadow.canonicalize().expect("canonical shadow");
    assert_eq!(
        sources.get(&shadow),
        Some(&BTreeSet::from(["crate::shadow".to_owned()]))
    );
    let error = audit_package_registration_sources("wow-world", &sources, &BTreeSet::new())
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

    let (sources, explicit_paths) =
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
    let error = audit_package_registration_sources("wow-world", &sources, &BTreeSet::new())
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
fn ownership_allows_only_the_exact_wow_handler_collector() {
    let fixture = source_graph_fixture("collector-owner");
    let crate_root = fixture.join("src/lib.rs");
    fs::create_dir_all(crate_root.parent().expect("crate root parent"))
        .expect("create collector source directory");
    let canonical_root = {
        fs::write(&crate_root, "inventory::collect!(PacketHandlerEntry);\n")
            .expect("write exact collector");
        crate_root.canonicalize().expect("canonical collector root")
    };
    let sources = BTreeMap::from([(canonical_root, BTreeSet::from(["crate".to_owned()]))]);
    let production_lib_roots = sources.keys().cloned().collect();

    audit_package_registration_sources("wow-handler", &sources, &production_lib_roots)
        .expect("one exact unconditional collector in the owner must pass");
    let error = audit_package_registration_sources("wow-handler", &sources, &BTreeSet::new())
        .expect_err(
            "a logical crate root that is not a Cargo lib target must not own the collector",
        );
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
        let error =
            audit_package_registration_sources("wow-handler", &sources, &production_lib_roots)
                .expect_err("collector mutant must fail closed");
        assert!(
            error.contains(expected_error),
            "{name}: expected {expected_error:?}, got {error:?}"
        );
    }

    fs::write(&crate_root, "inventory::collect!(PacketHandlerEntry);\n")
        .expect("restore exact collector");
    let error = audit_package_registration_sources("world-server", &sources, &production_lib_roots)
        .expect_err("the exact collector is forbidden outside wow-handler");
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
        let fixture = source_graph_fixture(name);
        let crate_root = fixture.join("src/lib.rs");
        let collector_module = fixture.join("src/registry.rs");
        fs::create_dir_all(crate_root.parent().expect("crate root parent"))
            .expect("create collector fixture");
        fs::write(&crate_root, format!("{parent_attribute}\nmod registry;\n"))
            .expect("write conditional collector parent");
        fs::write(
            &collector_module,
            "inventory::collect!(PacketHandlerEntry);\n",
        )
        .expect("write nested collector");

        let (sources, _) = audit_package_source_graph(&fixture, std::slice::from_ref(&crate_root))
            .expect("cfg-independent graph follows collector module");
        let production_lib_roots =
            BTreeSet::from([crate_root.canonicalize().expect("canonical lib root")]);
        let error =
            audit_package_registration_sources("wow-handler", &sources, &production_lib_roots)
                .expect_err("a collector below a parent module is not the lib-root collector");
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

#[test]
fn source_guard_rejects_path_grammar_it_cannot_resolve_exactly() {
    for (source, expected_error) in [
        (
            r#"#[cfg_attr(windows, path = "hidden.rs")] mod hidden;"#,
            "module #[cfg_attr(..., path = ...)] is not allowed",
        ),
        (
            r#"#[path = "hidden.rs"] mod inline { pub fn visible() {} }"#,
            "inline module inline",
        ),
        (
            r#"mod inline { #[path = "hidden.rs"] mod hidden; }"#,
            "declared inside an inline module",
        ),
    ] {
        let error = analyze_inline_source(source)
            .expect_err("ambiguous #[path] grammar must fail in the handler analyzer");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_rejects_submit_imports_and_alias_generators() {
    for (source, expected_error) in [
        (
            r#"
                use inv::submit as s;
                fn hidden() { s! { E { opcode: ClientOpcodes::Hidden } } }
            "#,
            "can alias an inventory registration macro",
        ),
        (
            r#"
                macro_rules! hidden {
                    () => { inv::submit! { E { opcode: ClientOpcodes::Hidden } } };
                }
                const INSTALL: () = { hidden!(); };
            "#,
            "handler-capable macro hidden",
        ),
        (
            r#"
                macro_rules! hidden {
                    () => {
                        use inventory::submit as s;
                        s! { E { opcode: ClientOpcodes::Hidden } }
                    };
                }
                const INSTALL: () = { hidden!(); };
            "#,
            "aliases/imports an inventory registration macro",
        ),
    ] {
        let error = analyze_inline_source(source)
            .expect_err("submit aliases and forwarding macros must fail closed");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_rejects_cfg_on_direct_macro_definition_invocation_and_ancestor() {
    let cases = [
        (
            r#"
                #[cfg(debug_assertions)]
                inventory::submit! {
                    PacketHandlerEntry {
                        opcode: ClientOpcodes::Alpha,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "alpha",
                    }
                }
            "#,
            "handler registration submit! is conditionally compiled",
        ),
        (
            r#"
                #[cfg(feature = "conditional-handler")]
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
                register_handler!(Alpha);
            "#,
            "registration macro register_handler is conditionally compiled",
        ),
        (
            r#"
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
                #[cfg(target_os = "linux")]
                register_handler!(Alpha);
            "#,
            "handler registration register_handler! is conditionally compiled",
        ),
        (
            r#"
                #[cfg(not(debug_assertions))]
                mod release_only {
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::Alpha,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "alpha",
                        }
                    }
                }
            "#,
            "handler registration submit! is conditionally compiled",
        ),
    ];

    for (source, expected_error) in cases {
        let error =
            analyze_inline_source(source).expect_err("conditional registration must be rejected");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_rejects_cfg_hidden_inside_a_registration_macro() {
    let error = analyze_inline_source(
        r#"
            macro_rules! register_handler {
                ($opcode:ident) => {
                    #[cfg_attr(debug_assertions, allow(dead_code))]
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
            register_handler!(Alpha);
        "#,
    )
    .expect_err("conditional tokens inside a registration macro must be rejected");

    assert!(
        error.contains("registration macro register_handler contains cfg/cfg_attr tokens"),
        "{error}"
    );
}

#[test]
fn source_guard_rejects_nested_conditional_registration_and_unresolved_include() {
    let nested_error = analyze_inline_source(
        r#"
            #[cfg(target_os = "linux")]
            const REGISTER: () = {
                inventory::submit! {
                    PacketHandlerEntry {
                        opcode: ClientOpcodes::Alpha,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "alpha",
                    }
                }
            };
        "#,
    )
    .expect_err("nested conditional registration must be rejected");
    assert!(
        nested_error.contains("registration grammar is allowed only at module item level"),
        "{nested_error}"
    );

    let nested_forwarder_error = analyze_inline_source(
        r#"
            type Hidden = PacketHandlerEntry;
            #[cfg(windows)]
            const REGISTER: () = {
                external_forward!(
                    inventory::submit,
                    Hidden {
                        opcode: ClientOpcodes::Alpha,
                    }
                );
            };
        "#,
    )
    .expect_err("a nested macro must not forward an inventory registration path");
    assert!(
        nested_forwarder_error.contains("external_forward")
            && nested_forwarder_error
                .contains("registration grammar is allowed only at module item level"),
        "{nested_forwarder_error}"
    );

    let include_error = analyze_inline_source(r#"include!("generated_handlers.rs");"#)
        .expect_err("unresolved source inclusion must be rejected");
    assert!(
        include_error.contains("unsupported item-level macro include!"),
        "{include_error}"
    );
}

#[test]
fn source_guard_rejects_handler_grammar_inside_blocks() {
    let cases = [
        (
            r#"
                fn hidden() {
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::Alpha,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "hidden",
                        }
                    }
                }
            "#,
            "inventory::submit!",
        ),
        (
            r#"
                macro_rules! register_handler {
                    ($opcode:ident) => {
                        inventory::submit! {
                            PacketHandlerEntry {
                                opcode: ClientOpcodes::$opcode,
                                status: SessionStatus::LoggedIn,
                                processing: PacketProcessing::Inplace,
                                handler_name: "hidden",
                            }
                        }
                    };
                }
                fn hidden() {
                    register_handler!(Alpha);
                }
            "#,
            "register_handler!",
        ),
        (
            r#"
                fn hidden() {
                    include!("generated_handlers.rs");
                }
            "#,
            "include!",
        ),
        (
            r#"
                fn hidden() {
                    submit_alias! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::Alpha,
                        }
                    }
                }
            "#,
            "submit_alias!",
        ),
        (
            r#"
                fn hidden() {
                    submit! {
                        E {
                            opcode: ClientOpcodes::Alpha,
                        }
                    }
                }
            "#,
            "submit!",
        ),
        (
            r#"
                fn hidden() {
                    inventory::collect! {
                        E
                    }
                }
            "#,
            "inventory::collect!",
        ),
        (
            r#"
                fn hidden() {
                    inv::__do_submit! {
                        E {
                            opcode: ClientOpcodes::Alpha,
                        }
                    }
                }
            "#,
            "inv::__do_submit!",
        ),
    ];

    for (source, macro_name) in cases {
        let error =
            analyze_inline_source(source).expect_err("nested handler grammar must fail closed");
        assert!(
            error.contains("registration grammar is allowed only at module item level"),
            "{error}"
        );
        assert!(error.contains(macro_name), "{error}");
    }
}

#[test]
fn source_guard_rejects_collector_inside_handler_owner() {
    let error = analyze_inline_source("inventory::collect!(PacketHandlerEntry);")
        .expect_err("the collector belongs only to the wow-handler package root");
    assert!(
        error.contains("unsupported item-level macro inventory::collect!"),
        "{error}"
    );
}

#[test]
fn source_guard_rejects_unknown_item_macro_that_can_expand_a_cfg_module() {
    let error = analyze_inline_source(
        r#"
            macro_rules! m {
                () => {
                    #[cfg(windows)]
                    mod windows_handlers;
                };
            }
            m!();
        "#,
    )
    .expect_err("unknown item macros must fail closed before expansion");
    assert!(error.contains("unsupported item-level macro m!"), "{error}");
}

#[test]
fn source_guard_rejects_nested_handler_macro_definition() {
    let error = analyze_inline_source(
        r#"
            fn install_conditionally() {
                #[cfg(windows)]
                macro_rules! hidden_handler {
                    ($opcode:ident) => {
                        inventory::submit! {
                            PacketHandlerEntry {
                                opcode: ClientOpcodes::$opcode,
                                status: SessionStatus::LoggedIn,
                                processing: PacketProcessing::Inplace,
                                handler_name: "hidden",
                            }
                        }
                    };
                }
                hidden_handler!(Alpha);
            }
        "#,
    )
    .expect_err("nested handler-capable macro definition must fail closed");
    assert!(
        error.contains("nested macro_rules! hidden_handler may generate a handler registration"),
        "{error}"
    );

    for (source, macro_name) in [
        (
            r#"
                fn define_local_generator() {
                    macro_rules! hidden_module {
                        () => {
                            #[cfg(windows)]
                            mod windows_handlers;
                        };
                    }
                }
            "#,
            "hidden_module",
        ),
        (
            r#"
                fn define_local_generator() {
                    macro_rules! hidden_include {
                        () => {
                            include!("generated_handlers.rs");
                        };
                    }
                }
            "#,
            "hidden_include",
        ),
    ] {
        let error = analyze_inline_source(source)
            .expect_err("nested module/include source generator must fail closed");
        assert!(
            error.contains(&format!(
                "nested macro_rules! {macro_name} may generate a handler registration"
            )),
            "{error}"
        );
    }
}

#[test]
fn source_guard_rejects_repeating_or_multi_arm_registration_macros() {
    let repeating = analyze_inline_source(
        r#"
            macro_rules! register_handler {
                ($($opcode:ident),+) => {
                    $(inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                        }
                    })+
                };
            }
            register_handler!(Alpha, Beta);
        "#,
    )
    .expect_err("repeating registration expansion must be rejected");
    assert!(
        repeating.contains("contains a macro repetition"),
        "{repeating}"
    );

    let multi_arm = analyze_inline_source(
        r#"
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
                () => {};
            }
            register_handler!(Alpha);
        "#,
    )
    .expect_err("multi-arm registration expansion must be rejected");
    assert!(multi_arm.contains("has 2 rule arms"), "{multi_arm}");
}
