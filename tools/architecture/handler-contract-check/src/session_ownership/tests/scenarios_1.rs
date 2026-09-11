//! Session ownership inventory regression scenarios, part 1 of 1.
//!
//! Moved out of the session_ownership.rs root under #660; every test is unchanged.

use super::*;

#[test]
fn rendering_a_policy_rejects_an_unreadable_or_invalid_snapshot() {
    let repository_root = crate::repository_root().expect("repository root");

    let missing = repository_root.join("tools/architecture/no-such-snapshot.json");
    let error = print_repository_persistence_policy_from_snapshot(&missing)
        .expect_err("a missing snapshot cannot render a policy");
    assert!(
        error.starts_with("cannot read") && error.contains("no-such-snapshot.json"),
        "unexpected error: {error}"
    );

    let not_json = repository_root.join("AGENTS.md");
    let error = print_repository_persistence_policy_from_snapshot(&not_json)
        .expect_err("a non-JSON snapshot cannot render a policy");
    assert!(
        error.starts_with("invalid persistence access snapshot") && error.contains("AGENTS.md"),
        "unexpected error: {error}"
    );
}

#[test]
fn checked_persistence_policy_matches_the_checked_snapshot() {
    let repository_root = crate::repository_root().expect("repository root");
    let rendered = print_repository_persistence_policy_from_snapshot(
        &repository_root.join(PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH),
    )
    .expect("render policy from the checked snapshot");
    let rendered: serde_json::Value =
        serde_json::from_str(&rendered).expect("rendered policy is JSON");
    let checked = fs::read_to_string(repository_root.join(PERSISTENCE_POLICY_RELATIVE_PATH))
        .expect("read checked policy");
    let checked: serde_json::Value =
        serde_json::from_str(&checked).expect("checked policy is JSON");
    assert_eq!(
        checked, rendered,
        "checked persistence policy disagrees with the checked snapshot; \
         regenerate it with print-persistence-policy --from-snapshot"
    );
}

fn unit(role: PackageRole, source_path: &str, source: &str) -> SourceUnit {
    SourceUnit {
        role,
        source_path: PathBuf::from(source_path),
        repository_relative_path: source_path.to_owned(),
        logical_module_path: "crate".to_owned(),
        cfg: Vec::new(),
        availability: Availability {
            production: true,
            test: true,
        },
        source: source.to_owned(),
    }
}

fn synthetic_baseline_with_network(
    world: &str,
    server: &str,
    network: &str,
) -> Result<SessionSyntaxBaseline, String> {
    collect_units(
        vec![
            unit(PackageRole::World, "wow-world/src/lib.rs", world),
            unit(PackageRole::Server, "world-server/src/main.rs", server),
            unit(PackageRole::Network, "wow-network/src/lib.rs", network),
        ],
        PersistenceAccessBaseline {
            schema_version: 3,
            accesses: Vec::new(),
        },
    )
}

fn synthetic_baseline(world: &str, server: &str) -> Result<SessionSyntaxBaseline, String> {
    synthetic_baseline_with_network(
        world,
        server,
        r#"
            pub enum SessionCommand { Kick(KickCommand) }
            pub struct KickCommand { pub reason: String }
        "#,
    )
}

#[test]
fn test_only_external_modules_supply_bridge_import_context() {
    let world = format!(
        "{} #[cfg(test)] mod test_fixture; \
         #[cfg(test)] mod checks {{ \
           use crate::test_fixture::Entity; \
           fn bridge(old: &wow_world::SharedMapManager, new: &Entity) {{}} \
         }}",
        world_source("", ""),
    );
    let mut fixture = unit(
        PackageRole::World,
        "wow-world/src/test_fixture.rs",
        "pub type Entity = wow_entities::Creature;",
    );
    fixture.logical_module_path = "crate::test_fixture".to_owned();
    fixture.cfg = vec!["cfg(test)".to_owned()];
    fixture.availability = Availability {
        production: false,
        test: true,
    };
    let baseline = collect_units(
        vec![
            unit(PackageRole::World, "wow-world/src/lib.rs", &world),
            fixture,
            unit(PackageRole::Server, "world-server/src/main.rs", &server_source("", "")),
            unit(PackageRole::Network, "wow-network/src/lib.rs",
                "pub enum SessionCommand { Kick(KickCommand) } pub struct KickCommand { pub reason: String }"),
        ],
        PersistenceAccessBaseline { schema_version: 3, accesses: Vec::new() },
    ).expect("top-level test-only mounts must not be dropped from bridge provenance");
    assert_eq!(baseline.bridge_accesses.bridges.len(), 1);
    assert_eq!(baseline.bridge_accesses.bridges[0].module, "crate::checks");
    assert_eq!(baseline.bridge_accesses.bridges[0].cfg, vec!["cfg (test)"]);
}

fn world_source(field: &str, extra_impl_item: &str) -> String {
    format!(
        r#"
            pub mod session {{
                pub struct WorldSession {{
                    pub account_id: u32,
                    {field}
                }}
                impl WorldSession {{
                    pub fn new(account_id: u32) -> Self {{ todo!() }}
                    pub fn set_account_id(&mut self, account_id: u32) {{}}
                    fn internal(&self) {{}}
                }}
            }}
            mod handlers {{
                impl crate::session::WorldSession {{
                    fn handle_packet(&mut self) {{}}
                    {extra_impl_item}
                }}
            }}
        "#
    )
}

fn server_source(extra_field: &str, extra_factory: &str) -> String {
    format!(
        r#"
            mod session_resources {{
                pub(super) struct SessionResources {{
                    pub(super) char_db: Option<u32>,
                    {extra_field}
                }}
            }}
            use session_resources::SessionResources;
            struct WorldSession;
            impl WorldSession {{
                fn new(_account_id: u32) -> Self {{ Self }}
                fn set_char_db(&mut self, _db: u32) {{}}
            }}
            mod session_factory {{
                use super::*;

                async fn create_session(account_id: u32, resources: SessionResources) {{
                    let mut session = WorldSession::new(account_id);
                    if let Some(db) = resources.char_db {{ session.set_char_db(db); }}
                    {extra_factory}
                }}
                async fn bootstrap() {{
                    let resources = SessionResources {{ char_db: None, {extra_field_init} }};
                    create_session(1, resources).await;
                }}
            }}
        "#,
        extra_field_init = if extra_field.is_empty() {
            ""
        } else {
            "extra: 0,"
        },
    )
}

#[test]
fn exact_field_sets_reject_growth_and_same_count_substitution() {
    let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
        .expect("baseline parses");
    let growth = synthetic_baseline(
        &world_source("state: u8, added: bool,", ""),
        &server_source("", ""),
    )
    .expect("growth parses");
    let error = compare_baseline(&baseline, &growth).expect_err("field growth must fail");
    assert!(
        error.contains("unreviewed WorldSession field surface"),
        "{error}"
    );

    let substitution = synthetic_baseline(
        &world_source("replacement: u8,", ""),
        &server_source("", ""),
    )
    .expect("substitution parses");
    let error = compare_baseline(&baseline, &substitution).expect_err("same-count swap must fail");
    assert!(
        error.contains("obsolete WorldSession field baseline"),
        "{error}"
    );
    assert!(
        error.contains("unreviewed WorldSession field surface"),
        "{error}"
    );
}

#[test]
fn external_impl_visible_and_setter_surfaces_are_exact() {
    let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
        .expect("baseline parses");
    let changed = synthetic_baseline(
        &world_source(
            "state: u8,",
            "pub(crate) fn set_external_state(&mut self, _state: u8) {}",
        ),
        &server_source("", ""),
    )
    .expect("changed surface parses");
    let error = compare_baseline(&baseline, &changed).expect_err("new impl item must fail");
    assert!(
        error.contains("unreviewed WorldSession impl item surface"),
        "{error}"
    );
    assert!(error.contains("external_impl"), "{error}");
    assert!(error.contains("setter"), "{error}");
    assert!(error.contains("visible"), "{error}");

    let renamed_private = world_source("state: u8,", "")
        .replace("fn internal(&self) {}", "fn renamed_internal(&self) {}");
    let changed = synthetic_baseline(&renamed_private, &server_source("", ""))
        .expect("private item change parses");
    let error = compare_baseline(&baseline, &changed)
        .expect_err("private inherent items in crate::session are exact too");
    assert!(
        error.contains("obsolete WorldSession impl item baseline"),
        "{error}"
    );
    assert!(
        error.contains("unreviewed WorldSession impl item surface"),
        "{error}"
    );
}

#[test]
fn cfg_test_items_are_classified_and_target_cfg_items_are_guarded() {
    let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
        .expect("baseline parses");
    let test_only = synthetic_baseline(
        &world_source(
            "state: u8, #[cfg(test)] fixture_only: bool,",
            "#[cfg(test)] fn test_helper(&self) {}",
        ),
        &server_source("", ""),
    )
    .expect("test-only surface parses");
    let error = compare_baseline(&baseline, &test_only)
        .expect_err("test fixtures are an explicit baseline surface");
    assert!(
        error.contains(r#""source_class":"test_fixture""#),
        "{error}"
    );

    let target_cfg = synthetic_baseline(
        &world_source(
            "state: u8, #[cfg(windows)] platform_state: bool,",
            "#[cfg(windows)] fn platform_handler(&self) {}",
        ),
        &server_source("", ""),
    )
    .expect("target cfg surface parses");
    let error = compare_baseline(&baseline, &target_cfg).expect_err("target cfg can be production");
    assert!(error.contains("cfg (windows)"), "{error}");
}

fn attributes(source: &str) -> Vec<syn::Attribute> {
    let syntax = syn::parse_file(source).expect("cfg fixture parses as Rust");
    match &syntax.items[0] {
        Item::Struct(item) => item.attrs.clone(),
        _ => panic!("expected struct cfg fixture"),
    }
}

#[test]
fn cfg_production_satisfiability_is_correlated_and_malformed_cfg_attr_fails() {
    let cfg_test = attributes("#[cfg(test)] struct Fixture;");
    assert!(!cfg_context_allows_production(&[], &cfg_test).unwrap());

    let any_test_feature = attributes(r#"#[cfg(any(test, feature = "fixture"))] struct Fixture;"#);
    assert!(cfg_context_allows_production(&[], &any_test_feature).unwrap());

    let not_test = attributes("#[cfg(not(test))] struct Fixture;");
    assert!(cfg_context_allows_production(&[], &not_test).unwrap());

    let contradictory = attributes("#[cfg(fixture)] #[cfg(not(fixture))] struct Fixture;");
    assert!(!cfg_context_allows_production(&[], &contradictory).unwrap());

    let all_contradictory = attributes("#[cfg(all(fixture, not(fixture)))] struct Fixture;");
    assert!(!cfg_context_allows_production(&[], &all_contradictory).unwrap());

    let parent = vec!["cfg (fixture)".to_owned()];
    let child = attributes("#[cfg(not(fixture))] struct Fixture;");
    assert!(!cfg_context_allows_production(&parent, &child).unwrap());

    let malformed_cfg_attr = attributes("#[cfg_attr(fixture)] struct Fixture;");
    let error = cfg_context_allows_production(&[], &malformed_cfg_attr)
        .expect_err("cfg_attr without an attribute must fail closed");
    assert!(
        error.contains("at least one conditional attribute"),
        "{error}"
    );

    let production_applies_test = attributes("#[cfg_attr(not(test), cfg(test))] struct Fixture;");
    assert!(!cfg_context_allows_production(&[], &production_applies_test).unwrap());
}

#[test]
fn aliases_and_macros_cannot_hide_world_session_impls() {
    let alias_world = format!(
        "{}\nuse crate::session::WorldSession as HiddenSession;",
        world_source("state: u8,", "")
    );
    let error = synthetic_baseline(&alias_world, &server_source("", ""))
        .expect_err("renamed WorldSession must fail closed");
    assert!(error.contains("renames WorldSession"), "{error}");

    let macro_world = format!(
        "{}\nmake_impl!(WorldSession);",
        world_source("state: u8,", "")
    );
    let error = synthetic_baseline(&macro_world, &server_source("", ""))
        .expect_err("macro target must fail closed");
    assert!(
        error.contains("macro-generated ownership surfaces"),
        "{error}"
    );
}

#[test]
fn include_macros_fail_closed_and_codegen_attributes_are_exact() {
    let include_world = format!(
        "{}\ninclude!(\"generated_session_surface.rs\");",
        world_source("state: u8,", "")
    );
    let error = synthetic_baseline(&include_world, &server_source("", ""))
        .expect_err("include inputs must fail closed");
    assert!(error.contains("include! macro invocation"), "{error}");

    let world = world_source("state: u8,", "");
    let server = server_source("", "");
    let baseline = synthetic_baseline_with_network(
        &world,
        &server,
        r#"
            pub enum SessionCommand { Kick(KickCommand) }
            pub struct KickCommand { pub reason: String }
        "#,
    )
    .expect("baseline parses");
    let generated = synthetic_baseline_with_network(
        &world,
        &server,
        r#"
            #[derive(Clone)]
            pub enum SessionCommand { Kick(KickCommand) }
            pub struct KickCommand { pub reason: String }
        "#,
    )
    .expect("generated input parses");
    assert_eq!(generated.generated_surface_inputs.len(), 1);
    let error = compare_baseline(&baseline, &generated)
        .expect_err("new derive input must be reviewed exactly");
    assert!(
        error.contains("unreviewed generated ownership input"),
        "{error}"
    );
    assert!(error.contains(r#""kind":"derive""#), "{error}");
}

#[test]
fn splitting_an_impl_block_does_not_change_the_logical_owner_surface() {
    let baseline_world = r#"
        pub mod session {
            pub struct WorldSession { pub account_id: u32 }
            impl WorldSession {
                fn first(&self) {}
                fn second(&self) {}
            }
        }
    "#;
    let split_world = r#"
        pub mod session {
            pub struct WorldSession { pub account_id: u32 }
            impl WorldSession { fn first(&self) {} }
            impl WorldSession { fn second(&self) {} }
        }
    "#;
    let baseline =
        synthetic_baseline(baseline_world, &server_source("", "")).expect("baseline parses");
    let split =
        synthetic_baseline(split_world, &server_source("", "")).expect("split surface parses");
    compare_baseline(&baseline, &split)
        .expect("physical impl block count is deliberately not a ratchet");
}

#[test]
fn splitting_a_private_owner_into_child_modules_keeps_one_logical_owner() {
    let baseline_world = r#"
        pub mod session { pub struct WorldSession { pub account_id: u32 } }
        mod handlers {
            mod misc {
                impl crate::session::WorldSession { fn calendar(&self) {} }
            }
        }
    "#;
    let split_world = r#"
        pub mod session { pub struct WorldSession { pub account_id: u32 } }
        mod handlers {
            mod misc {
                mod calendar {
                    impl crate::session::WorldSession { fn calendar(&self) {} }
                }
            }
        }
    "#;
    let baseline =
        synthetic_baseline(baseline_world, &server_source("", "")).expect("baseline parses");
    let split = synthetic_baseline(split_world, &server_source("", ""))
        .expect("private child module parses");

    compare_baseline(&baseline, &split)
        .expect("a private physical child remains part of its logical owner");
    assert_eq!(split.world_session.impls.len(), 1);
    assert_eq!(split.world_session.impls[0].module, "crate::handlers::misc");
}

#[test]
fn session_resources_and_factory_fanout_are_exact() {
    let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
        .expect("baseline parses");
    let resources_growth = synthetic_baseline(
        &world_source("state: u8,", ""),
        &server_source("pub(super) extra: u32,", ""),
    )
    .expect("resource growth parses");
    let error = compare_baseline(&baseline, &resources_growth)
        .expect_err("SessionResources growth must fail");
    assert!(
        error.contains("unreviewed SessionResources field surface"),
        "{error}"
    );

    let factory_growth = synthetic_baseline(
        &world_source("state: u8,", ""),
        &server_source("", "session.set_new_resource(1);"),
    )
    .expect("factory growth parses");
    let error = compare_baseline(&baseline, &factory_growth).expect_err("factory growth must fail");
    assert!(error.contains("create_session setter call"), "{error}");

    let alias_factory = synthetic_baseline(
        &world_source("state: u8,", ""),
        &server_source(
            "",
            "let alias = &mut session; alias.install_new_resource(1);",
        ),
    )
    .expect("alias receiver parses");
    assert!(
        alias_factory
            .session_factory
            .setter_call_sites
            .iter()
            .any(|call| call.callee == "alias.install_new_resource"),
        "setter/install calls through aliases must be captured"
    );
}

#[test]
fn factory_body_and_session_bearing_helper_bodies_are_exact() {
    let world = world_source("state: u8,", "");
    let baseline_server = server_source("", "");
    let baseline = synthetic_baseline(&world, &baseline_server).expect("baseline parses");

    let helper_wiring = server_source("", "wire_extra(&mut session);").replacen(
        "async fn create_session",
        "fn wire_extra(session: &mut WorldSession) { session.set_char_db(7); }\n\
         async fn create_session",
        1,
    );
    let helper_wiring = synthetic_baseline(&world, &helper_wiring).expect("helper wiring parses");
    assert_eq!(
        baseline.session_factory.setter_call_sites, helper_wiring.session_factory.setter_call_sites,
        "a setter delegated to a helper is deliberately outside the direct-call inventory"
    );
    let error = compare_baseline(&baseline, &helper_wiring)
        .expect_err("new helper wiring must change the full factory body fingerprint");
    assert!(
        error.contains("create_session body fingerprint changed"),
        "{error}"
    );

    let same_signature_body_change = baseline_server.replace(
        "WorldSession::new(account_id)",
        "WorldSession::new(account_id.wrapping_add(1))",
    );
    let same_signature_body_change = synthetic_baseline(&world, &same_signature_body_change)
        .expect("same-signature body change parses");
    assert_eq!(
        baseline.session_factory.signature,
        same_signature_body_change.session_factory.signature
    );
    assert_eq!(
        baseline.session_factory.world_session_new_sites,
        same_signature_body_change
            .session_factory
            .world_session_new_sites
    );
    let error = compare_baseline(&baseline, &same_signature_body_change)
        .expect_err("same-signature factory body changes must drift");
    assert!(
        error.contains("create_session body fingerprint changed"),
        "{error}"
    );

    let helper_v1 = server_source("", "wire_extra(&mut session);").replacen(
        "async fn create_session",
        "fn wire_extra(session: &mut WorldSession) { session.set_char_db(1); }\n\
         async fn create_session",
        1,
    );
    let helper_v2 = helper_v1.replace("session.set_char_db(1)", "session.set_char_db(2)");
    let helper_v1 = synthetic_baseline(&world, &helper_v1).expect("helper v1 parses");
    let helper_v2 = synthetic_baseline(&world, &helper_v2).expect("helper v2 parses");
    assert_eq!(
        helper_v1.session_factory.body_fingerprint, helper_v2.session_factory.body_fingerprint,
        "only the already-wired helper body changed"
    );
    let error = compare_baseline(&helper_v1, &helper_v2)
        .expect_err("a helper called with session must have an exact body fingerprint");
    assert!(
        error.contains("create_session session-bearing helper body"),
        "{error}"
    );
}

#[test]
fn command_variants_and_transitive_payloads_are_exact() {
    let world = world_source("state: u8,", "");
    let server = server_source("", "");
    let baseline = synthetic_baseline_with_network(
        &world,
        &server,
        r#"
            pub enum SessionCommand { Kick(KickCommand) }
            pub struct KickCommand { pub nested: NestedPayload }
            pub struct NestedPayload { pub reason: String }
        "#,
    )
    .expect("network baseline parses");
    let changed = synthetic_baseline_with_network(
        &world,
        &server,
        r#"
            pub enum SessionCommand { Kick(KickCommand), Refresh }
            pub struct KickCommand { pub nested: NestedPayload }
            pub struct NestedPayload { pub reason: Vec<u8> }
        "#,
    )
    .expect("changed network surface parses");
    let error = compare_baseline(&baseline, &changed).expect_err("network drift must fail");
    assert!(error.contains("SessionCommand variants changed"), "{error}");
    assert!(error.contains("transitive payload type"), "{error}");
}

#[test]
fn direct_registry_accesses_are_production_only_and_exact() {
    let world = world_source("state: u8,", "");
    let server = server_source("", "");
    let baseline = synthetic_baseline(&world, &server).expect("baseline parses");

    let test_only_world = format!(
        "{world}\n#[cfg(test)] fn fixture(players: &PlayerRegistry) {{ players.clear(); }}"
    );
    let test_only =
        synthetic_baseline(&test_only_world, &server).expect("test-only registry access parses");
    compare_baseline(&baseline, &test_only)
        .expect("test-only registry access is not production debt");

    let production_world =
        format!("{world}\nfn escape(players: &PlayerRegistry) {{ players.get(&1); }}");
    let production =
        synthetic_baseline(&production_world, &server).expect("production registry access parses");
    let error = compare_baseline(&baseline, &production)
        .expect_err("new production registry access must fail the exact ratchet");
    assert!(
        error.contains("untracked direct registry access"),
        "{error}"
    );
    assert!(error.contains("PlayerRegistry"), "{error}");
}

#[test]
fn baseline_envelope_round_trips_with_semantic_policy_fields() {
    let baseline = synthetic_baseline(&world_source("state: u8,", ""), &server_source("", ""))
        .expect("baseline parses");
    let mut value = serde_json::to_value(BaselineEnvelope {
        schema_version: 1,
        persistence_access_snapshot: PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH,
        syntax_baseline: &baseline,
    })
    .expect("baseline serializes");
    value
        .as_object_mut()
        .expect("envelope object")
        .insert("responsibilities".to_owned(), serde_json::json!([]));
    let parsed: PolicyEnvelope =
        serde_json::from_value(value).expect("semantic keys remain isolated");
    assert_eq!(parsed.syntax_baseline, baseline);
}

#[test]
fn server_ownership_root_follows_private_library_modules() {
    let repository_root = crate::repository_root().expect("repository root");
    let units = repository_units(
        &repository_root,
        PackageRole::Server,
        SERVER_PACKAGE_ROOT,
        SERVER_CRATE_ROOT,
    )
    .expect("world-server library modules must be discoverable");

    assert!(units.iter().any(|unit| {
        unit.logical_module_path == SESSION_RESOURCES_MODULE
            && unit.source_path.ends_with("session_resources.rs")
    }));
    assert!(units.iter().any(|unit| {
        unit.logical_module_path == SESSION_FACTORY_MODULE
            && unit.source_path.ends_with("session_factory.rs")
    }));
    assert!(
        units
            .iter()
            .all(|unit| !unit.source_path.ends_with("main.rs"))
    );
}

/// The collector sees the repository, and what it sees agrees with the
/// baseline the ratchet enforces.
///
/// This used to restate scanner sizes as literals frozen when #181 first
/// baselined the scanner. Every one of those numbers is already owned by
/// `session-ownership-policy.json`, so the copies could only rot, and they
/// did: by #363 all but one were wrong. It asserts properties now, and the
/// counts stay in the one place that ratchets them.
///
/// It also asked for the persistence inventory it never looked at, which
/// cost a full workspace scan and is why it was skipped by name in both
/// validation profiles. The syntax surface is what it asserts, so the
/// syntax surface is what it collects.
#[test]
fn repository_surface_can_be_collected() {
    let repository_root = crate::repository_root().expect("repository root");
    let baseline = repository_syntax_for_tests()
        .unwrap_or_else(|error| panic!("repository baseline must parse:\n{error}"));

    let raw_session_source =
        fs::read_to_string(repository_root.join("crates/wow-world/src/session/mod.rs"))
            .expect("read session source");
    let raw_session = syn::parse_file(&raw_session_source).expect("parse session source");
    let raw_fields = raw_session
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(item) if item.ident == WORLD_SESSION_NAME => Some(&item.fields),
            _ => None,
        })
        .expect("WorldSession definition");

    // The collector reaches the same struct the parser does, field for field.
    let raw_names: BTreeSet<String> = raw_fields
        .iter()
        .map(|field| {
            field
                .ident
                .as_ref()
                .expect("WorldSession has named fields")
                .to_string()
        })
        .collect();
    let collected_names: BTreeSet<String> = baseline
        .world_session
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect();
    assert_eq!(collected_names, raw_names);

    // Production and test-fixture partition the surface: no field is both,
    // none is neither, and the split matches the cfg the source declares.
    let production = baseline
        .world_session
        .fields
        .iter()
        .filter(|field| field.source_class == "production")
        .count();
    let test_fixture = baseline
        .world_session
        .fields
        .iter()
        .filter(|field| field.source_class == "test_fixture")
        .count();
    assert_eq!(
        production + test_fixture,
        baseline.world_session.fields.len()
    );
    let raw_test_only = raw_fields
        .iter()
        .filter(|field| {
            !cfg_context_allows_production(&[], &field.attrs)
                .expect("repository field cfg is valid")
        })
        .count();
    assert_eq!(test_fixture, raw_test_only);

    // What it collected is what the checked-in baseline says it should be.
    // This is the same comparison the gate performs, so a stale test and a
    // stale ratchet can no longer disagree.
    let policy = load_policy(&repository_root.join(POLICY_RELATIVE_PATH))
        .expect("checked-in session ownership policy loads");
    compare_baseline(&policy.syntax_baseline, &baseline)
        .expect("the collected surface matches the checked-in baseline");

    // Every surface the baseline tracks was actually collected, so an empty
    // scan cannot pass the comparison by matching an empty baseline.
    assert!(!baseline.world_session.fields.is_empty());
    assert!(!baseline.world_session.impls.is_empty());
    assert!(!baseline.session_resources.fields.is_empty());
    assert!(!baseline.session_command.variants.is_empty());
    assert!(!baseline.registry_accesses.accesses.is_empty());
    assert!(baseline.registry_accesses.accesses.iter().all(|record| {
        record.source.starts_with("crates/") && !Path::new(&record.source).is_absolute()
    }));
}
