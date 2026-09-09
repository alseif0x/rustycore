//! Handler-contract regressions, part 1 of 3.
//!
//! Moved out of the tests.rs root under #660; every test is unchanged.

use super::*;

#[test]
fn repository_handler_contract_passes() {
    let report = check_repository()
        .unwrap_or_else(|error| panic!("invalid repository handler contract:\n{error}"));
    assert!(report.starts_with("handler contract: PASS"), "{report}");
    assert!(report.contains("one dispatch mechanism"), "{report}");
    let owners = "world-modules, world-server, wow-handler, wow-world)";
    assert!(report.contains(owners), "{report}");
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
            handler_calls: BTreeSet::new(),
            dispatches_through_registration: false,
        }
    );
}

#[test]
fn dispatcher_parser_detects_every_shape_that_reintroduces_an_opcode_arm() {
    // #359 left one rule, so these shapes no longer need one rejection each:
    // a `match opcode` arm in any form is the second declaration coming back.
    for source in [
        r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        match opcode {
                            ClientOpcodes::Alpha | _ => {}
                        }
                    }
                }
            "#,
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
        r#"
                impl WorldSession {
                    async fn dispatch_packet(&mut self) {
                        async fn nested() {
                            match opcode {
                                ClientOpcodes::Alpha => {}
                                _ => {}
                            }
                        }
                    }
                }
            "#,
    ] {
        let contract =
            dispatcher_contract_from_source(source).expect("an opcode arm parses as an arm");
        let error = assert_single_dispatch_mechanism(&contract)
            .expect_err("a reintroduced opcode arm must fail");
        assert!(
            error.contains("decides 1 opcode(s) by hand (Alpha)"),
            "{error}"
        );
    }

    // A duplicated opcode is still a parse-level error: the arm table it would
    // rebuild cannot even be read unambiguously.
    let error = dispatcher_contract_from_source(
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
    )
    .expect_err("a duplicate opcode arm must fail");
    assert!(error.contains("duplicate opcode arm Alpha"), "{error}");
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
    let unconditional: BTreeSet<_> = sources.keys().cloned().collect();
    let error =
        audit_package_registration_sources_with_owner("wow-world", &sources, &unconditional, owner)
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
fn single_mechanism_check_rejects_a_reintroduced_arm_or_direct_handler_call() {
    let ok = DispatcherContract {
        opcode_names: BTreeSet::new(),
        handler_calls: BTreeSet::new(),
        dispatches_through_registration: true,
    };
    assert!(
        assert_single_dispatch_mechanism(&ok).is_ok(),
        "one mechanism and the registered call must pass"
    );

    let arm_back = DispatcherContract {
        opcode_names: ["Alpha".to_owned()].into_iter().collect(),
        dispatches_through_registration: true,
        ..Default::default()
    };
    let error = assert_single_dispatch_mechanism(&arm_back)
        .expect_err("a reintroduced opcode arm must fail");
    assert!(
        error.contains("decides 1 opcode(s) by hand (Alpha)"),
        "{error}"
    );

    let handler_back = DispatcherContract {
        handler_calls: ["handle_alpha".to_owned()].into_iter().collect(),
        dispatches_through_registration: true,
        ..Default::default()
    };
    let error = assert_single_dispatch_mechanism(&handler_back)
        .expect_err("a direct handler call must fail");
    assert!(
        error.contains("calls 1 handler method(s) on self (handle_alpha)"),
        "{error}"
    );

    let error = assert_single_dispatch_mechanism(&DispatcherContract::default())
        .expect_err("a dispatcher that never calls the registration must fail");
    assert!(
        error.contains("never calls the registered handler"),
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
    let session_path = Path::new("crates/wow-world/src/session/mod.rs");
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
            "session/mod.rs invokes inventory registration macro inventory::submit! outside the \
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
            "session/mod.rs invokes audited handler registration macro register_move! outside the \
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
        include_error.contains(
            "session/mod.rs uses include! outside the declared handler-registration owner"
        ),
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
            Path::new("crates/wow-world/src/session/mod.rs"),
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
        Path::new("crates/wow-world/src/session/mod.rs"),
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
        Path::new("crates/wow-world/src/session/mod.rs"),
        invocation,
    )
    .expect_err("passing inventory::submit through an unknown macro must fail source audit");
    assert!(
        invocation_error.contains("passes an inventory registration path")
            && invocation_error.contains("external_forward"),
        "{invocation_error}"
    );

    let mount_error = reject_registration_syntax_outside_handlers(
        Path::new("crates/wow-world/src/session/mod.rs"),
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
