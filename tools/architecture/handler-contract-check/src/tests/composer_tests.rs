// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::ownership::{SourceMountContext, WorkspaceSourceMount};
use crate::registrations::{
    ACCOUNT_DATA_REGISTRAR, BANK_REGISTRAR, CALENDAR_REGISTRAR, CHAT_REGISTRAR,
    CLIENT_STATE_REGISTRAR,
    DIRECT_REGISTRAR_CONTRACTS, DirectRegistrarContract, EQUIPMENT_SET_USE_REGISTRAR,
    INSTANCES_REGISTRAR, INVENTORY_REGISTRAR, REPUTATION_REGISTRAR, RegistrarFacadeContract,
    ARENA_TEAM_REGISTRAR, BATTLENET_REGISTRAR, DATA_SERVICE_REGISTRAR, APPLICATION_GROUP_REGISTRAR, GUILD_REGISTRAR, QUEST_QUERY_REGISTRAR, COMBAT_REGISTRAR, PLAYER_QUERY_REGISTRAR, COLLECTIONS_REGISTRAR, SOCIAL_GROUP_REGISTRAR, SOCIAL_CONTACTS_REGISTRAR, SOCIAL_INSPECT_REGISTRAR, SUPPORT_REGISTRAR, validate_composition_mounts,
    validate_composition_mounts_with_contracts,
};

const SYNTHETIC_OWNER_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "handlers",
    exports: &["register_synthetic_handlers_like_cpp"],
}];
const SYNTHETIC_OWNER: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SyntheticFixtureOwner",
    package: "synthetic-fixture-owner",
    module: "crate::handlers",
    registrar: "register_synthetic_handlers_like_cpp",
    host_trait: "SyntheticFixtureHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: SYNTHETIC_OWNER_FACADES,
};

fn mount(
    package: &str,
    logical_module_path: &str,
    source_path: &str,
    source: &str,
) -> WorkspaceSourceMount {
    WorkspaceSourceMount {
        package: package.to_owned(),
        source_path: PathBuf::from(source_path),
        contexts: BTreeSet::from([SourceMountContext {
            logical_module_path: logical_module_path.to_owned(),
            cfg: Vec::new(),
            production_possible: true,
            test_possible: true,
        }]),
        source: source.to_owned(),
    }
}

fn actual_mounts() -> Vec<WorkspaceSourceMount> {
    vec![
        mount(
            "world-server",
            "crate::handler_registry",
            "crates/world-server/src/handler_registry.rs",
            include_str!("../../../../../crates/world-server/src/handler_registry.rs"),
        ),
        mount(
            "wow-world",
            "crate::session::registry",
            "crates/wow-world/src/session/registry.rs",
            include_str!("../../../../../crates/wow-world/src/session/registry.rs"),
        ),
        mount(
            "wow-world-inventory",
            "crate",
            "crates/wow-world-inventory/src/lib.rs",
            include_str!("../../../../../crates/wow-world-inventory/src/lib.rs"),
        ),
        mount(
            "wow-world-inventory",
            "crate::handlers",
            "crates/wow-world-inventory/src/handlers/mod.rs",
            include_str!("../../../../../crates/wow-world-inventory/src/handlers/mod.rs"),
        ),
        mount(
            "wow-world-inventory",
            "crate::handlers::equipment_sets",
            "crates/wow-world-inventory/src/handlers/equipment_sets.rs",
            include_str!(
                "../../../../../crates/wow-world-inventory/src/handlers/equipment_sets.rs"
            ),
        ),
        mount(
            "wow-world-application",
            "crate",
            "crates/wow-world-application/src/lib.rs",
            include_str!("../../../../../crates/wow-world-application/src/lib.rs"),
        ),
        mount(
            "wow-world-application",
            "crate::instances",
            "crates/wow-world-application/src/instances/mod.rs",
            include_str!("../../../../../crates/wow-world-application/src/instances/mod.rs"),
        ),
        mount(
            "wow-world-application",
            "crate::instances::registration",
            "crates/wow-world-application/src/instances/registration.rs",
            include_str!(
                "../../../../../crates/wow-world-application/src/instances/registration.rs"
            ),
        ),
        mount(
            EQUIPMENT_SET_USE_REGISTRAR.package,
            EQUIPMENT_SET_USE_REGISTRAR.module,
            "crates/wow-world-application/src/equipment_set_use.rs",
            include_str!("../../../../../crates/wow-world-application/src/equipment_set_use.rs"),
        ),
        mount(
            BANK_REGISTRAR.package,
            BANK_REGISTRAR.module,
            "crates/wow-world-application/src/bank.rs",
            include_str!("../../../../../crates/wow-world-application/src/bank.rs"),
        ),
        mount(
            REPUTATION_REGISTRAR.package,
            REPUTATION_REGISTRAR.module,
            "crates/wow-world-application/src/reputation.rs",
            include_str!("../../../../../crates/wow-world-application/src/reputation.rs"),
        ),
        mount(
            CLIENT_STATE_REGISTRAR.package,
            CLIENT_STATE_REGISTRAR.module,
            "crates/wow-world-application/src/client_state.rs",
            include_str!("../../../../../crates/wow-world-application/src/client_state.rs"),
        ),
        mount(
            CALENDAR_REGISTRAR.package,
            CALENDAR_REGISTRAR.module,
            "crates/wow-world-social/src/calendar_handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/calendar_handlers.rs"),
        ),
        mount(
            CHAT_REGISTRAR.package,
            CHAT_REGISTRAR.module,
            "crates/wow-world-social/src/chat_handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/chat_handlers.rs"),
        ),
        mount(
            COLLECTIONS_REGISTRAR.package,
            COLLECTIONS_REGISTRAR.module,
            "crates/wow-world-application/src/collections_handlers.rs",
            include_str!("../../../../../crates/wow-world-application/src/collections_handlers.rs"),
        ),
        mount(
            PLAYER_QUERY_REGISTRAR.package,
            PLAYER_QUERY_REGISTRAR.module,
            "crates/wow-world-application/src/player_query_handlers.rs",
            include_str!("../../../../../crates/wow-world-application/src/player_query_handlers.rs"),
        ),
        mount(
            COMBAT_REGISTRAR.package,
            COMBAT_REGISTRAR.module,
            "crates/wow-world-application/src/combat_handlers.rs",
            include_str!("../../../../../crates/wow-world-application/src/combat_handlers.rs"),
        ),
        mount(
            QUEST_QUERY_REGISTRAR.package,
            QUEST_QUERY_REGISTRAR.module,
            "crates/wow-world-application/src/quest_query_handlers.rs",
            include_str!("../../../../../crates/wow-world-application/src/quest_query_handlers.rs"),
        ),
        mount(
            GUILD_REGISTRAR.package,
            GUILD_REGISTRAR.module,
            "crates/wow-world-social/src/guild_handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/guild_handlers.rs"),
        ),
        mount(
            APPLICATION_GROUP_REGISTRAR.package,
            APPLICATION_GROUP_REGISTRAR.module,
            "crates/wow-world-application/src/group_handlers.rs",
            include_str!("../../../../../crates/wow-world-application/src/group_handlers.rs"),
        ),
        mount(
            SOCIAL_GROUP_REGISTRAR.package,
            SOCIAL_GROUP_REGISTRAR.module,
            "crates/wow-world-social/src/group_handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/group_handlers.rs"),
        ),
        mount(
            DATA_SERVICE_REGISTRAR.package,
            DATA_SERVICE_REGISTRAR.module,
            "crates/wow-world-application/src/data_service_handlers.rs",
            include_str!("../../../../../crates/wow-world-application/src/data_service_handlers.rs"),
        ),
        mount(
            BATTLENET_REGISTRAR.package,
            BATTLENET_REGISTRAR.module,
            "crates/wow-world-lifecycle/src/battlenet_handlers.rs",
            include_str!("../../../../../crates/wow-world-lifecycle/src/battlenet_handlers.rs"),
        ),
        mount(
            ARENA_TEAM_REGISTRAR.package,
            ARENA_TEAM_REGISTRAR.module,
            "crates/wow-world-social/src/arena_team_handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/arena_team_handlers.rs"),
        ),
        mount(
            SOCIAL_CONTACTS_REGISTRAR.package,
            SOCIAL_CONTACTS_REGISTRAR.module,
            "crates/wow-world-social/src/social_contacts_handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/social_contacts_handlers.rs"),
        ),
        mount(
            SOCIAL_INSPECT_REGISTRAR.package,
            "crate",
            "crates/wow-world-social/src/lib.rs",
            include_str!("../../../../../crates/wow-world-social/src/lib.rs"),
        ),
        mount(
            SOCIAL_INSPECT_REGISTRAR.package,
            SOCIAL_INSPECT_REGISTRAR.module,
            "crates/wow-world-social/src/handlers.rs",
            include_str!("../../../../../crates/wow-world-social/src/handlers.rs"),
        ),
        mount(
            ACCOUNT_DATA_REGISTRAR.package,
            "crate",
            "crates/wow-world-lifecycle/src/lib.rs",
            include_str!("../../../../../crates/wow-world-lifecycle/src/lib.rs"),
        ),
        mount(
            ACCOUNT_DATA_REGISTRAR.package,
            ACCOUNT_DATA_REGISTRAR.module,
            "crates/wow-world-lifecycle/src/handlers.rs",
            include_str!("../../../../../crates/wow-world-lifecycle/src/handlers.rs"),
        ),
        mount(
            SUPPORT_REGISTRAR.package,
            SUPPORT_REGISTRAR.module,
            "crates/wow-world-lifecycle/src/support.rs",
            include_str!("../../../../../crates/wow-world-lifecycle/src/support.rs"),
        ),
    ]
}

fn synthetic_registrar_source(contract: DirectRegistrarContract, opcode: &str) -> String {
    r#"
use wow_handler::{DuplicateHandlerRegistrationLikeCpp, PacketHandlerEntry, RegistryBuilder};

pub fn $REGISTRAR<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: $HOST<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::$OPCODE,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "synthetic_handler",
        handler: synthetic_handler,
    })?;
    Ok(())
}
"#
    .replace("$REGISTRAR", contract.registrar)
    .replace("$HOST", contract.host_trait)
    .replace("$OPCODE", opcode)
}

fn synthetic_two_owner_mounts() -> Vec<WorkspaceSourceMount> {
    let inventory_root = r#"
pub use handlers::{
    AuctionHandlerCxLikeCpp,
    ItemTextQueryHandlerCxLikeCpp,
    EquipmentSetsHandlerCxLikeCpp,
    InventoryHandlerHostLikeCpp,
    EquipmentSetsSaveCxLikeCpp,
    register_inventory_handlers_like_cpp,
};
"#;
    let inventory_handlers = r#"
pub use equipment_sets::{
    ItemTextQueryHandlerCxLikeCpp,
    EquipmentSetsHandlerCxLikeCpp,
    InventoryHandlerHostLikeCpp,
    register_inventory_handlers_like_cpp,
};
"#;
    let synthetic_root = "pub use handlers::register_synthetic_handlers_like_cpp;";
    let production_composer = r#"
use wow_world::session::registry::register_remaining_handlers_like_cpp;

/// Synthetic production composition for the finite two-owner test.
pub fn compose_packet_handlers_like_cpp() -> Result<
    Arc<WorldPacketHandlerRegistry>,
    DuplicateHandlerRegistrationLikeCpp,
> {
    let mut builder = WorldPacketHandlerRegistryBuilder::new();
    wow_world_inventory::register_inventory_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(&mut builder)?;
    synthetic_fixture_owner::register_synthetic_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(&mut builder)?;
    register_remaining_handlers_like_cpp(&mut builder)?;
    Ok(Arc::new(builder.build()))
}
"#;
    let fixture_composer = r#"
use wow_world::session::registry::register_remaining_handlers_like_cpp;

/// Synthetic fixture composition for the finite two-owner test.
#[must_use]
#[cfg(any(test, feature = "test-fixtures"))]
pub fn build_dispatch_table() -> Arc<WorldPacketHandlerRegistry> {
    let mut builder = WorldPacketHandlerRegistryBuilder::new();
    wow_world_inventory::register_inventory_handlers_like_cpp(&mut builder)
        .expect("invalid duplicate packet handler composition");
    synthetic_fixture_owner::register_synthetic_handlers_like_cpp(&mut builder)
        .expect("invalid duplicate packet handler composition");
    register_remaining_handlers_like_cpp(&mut builder)
        .expect("invalid duplicate packet handler composition");
    Arc::new(builder.build())
}
"#;

    vec![
        mount(
            "world-server",
            "crate::handler_registry",
            "synthetic/world-server/handler_registry.rs",
            production_composer,
        ),
        mount(
            "wow-world",
            "crate::session::registry",
            "synthetic/wow-world/session/registry.rs",
            fixture_composer,
        ),
        mount(
            INVENTORY_REGISTRAR.package,
            "crate",
            "synthetic/inventory/lib.rs",
            inventory_root,
        ),
        mount(
            INVENTORY_REGISTRAR.package,
            "crate::handlers",
            "synthetic/inventory/handlers/mod.rs",
            inventory_handlers,
        ),
        mount(
            INVENTORY_REGISTRAR.package,
            INVENTORY_REGISTRAR.module,
            "synthetic/inventory/handlers/equipment_sets.rs",
            &synthetic_registrar_source(INVENTORY_REGISTRAR, "InventoryOnly"),
        ),
        mount(
            SYNTHETIC_OWNER.package,
            "crate",
            "synthetic/fixture-owner/lib.rs",
            synthetic_root,
        ),
        mount(
            SYNTHETIC_OWNER.package,
            SYNTHETIC_OWNER.module,
            "synthetic/fixture-owner/handlers.rs",
            &synthetic_registrar_source(SYNTHETIC_OWNER, "FixtureOnly"),
        ),
    ]
}

/// Apply a fixture mutation, failing loudly when the exact source no longer matches.
fn mutate_fixture(source: &str, from: &str, to: &str) -> String {
    let mutated = source.replace(from, to);
    assert_ne!(mutated, source, "fixture mutation did not match: {from:?}");
    mutated
}

fn assert_rejected(mounts: &[WorkspaceSourceMount], case: &str) {
    assert!(
        validate_composition_mounts(mounts).is_err(),
        "composition grammar accepted {case}"
    );
}

#[test]
fn composition_guard_accepts_a_finite_two_owner_synthetic_fixture() {
    validate_composition_mounts_with_contracts(
        &synthetic_two_owner_mounts(),
        &[INVENTORY_REGISTRAR, SYNTHETIC_OWNER],
    )
    .expect("the synthetic two-owner fixture is independent of actual composer sources");
}

#[test]
fn composition_guard_accepts_actual_production_fixture_and_both_exact_facades() {
    validate_composition_mounts(&actual_mounts())
        .expect("actual normal composer, fixture dispatch, and all owner facades match");
}

#[test]
fn composition_guard_requires_bank_once_in_both_builders_with_exact_facade() {
    let actual = actual_mounts();
    for index in [0, 1, 5] {
        let mut missing = actual.clone();
        missing[index].source = missing[index].source.replace(
            "register_bank_handlers_like_cpp",
            "unowned_bank_registration",
        );
        assert_rejected(&missing, "missing Bank composition or facade");
    }
    let mut aliased = actual.clone();
    aliased[0].source = aliased[0].source.replace(
        "wow_world_application::register_bank_handlers_like_cpp",
        "other_application::register_bank_handlers_like_cpp",
    );
    assert_rejected(&aliased, "aliased Bank provider");
    let mut duplicate = actual;
    duplicate[0].source = duplicate[0].source.replace(
        "    register_remaining_handlers_like_cpp(&mut builder)?;",
        "    wow_world_application::register_bank_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(&mut builder)?;\n    register_remaining_handlers_like_cpp(&mut builder)?;",
    );
    assert_rejected(&duplicate, "duplicate Bank registrar call");
}

#[test]
fn composition_guard_rejects_missing_aliased_and_wrong_equipment_set_use_calls() {
    let mut missing = actual_mounts();
    missing[0].source = missing[0].source.replace(
        "register_equipment_set_use_handler_like_cpp",
        "unowned_equipment_set_use_handler_like_cpp",
    );
    assert_rejected(&missing, "missing production EquipmentSetUse registrar");

    let mut missing_fixture = actual_mounts();
    missing_fixture[1].source = missing_fixture[1].source.replace(
        "register_equipment_set_use_handler_like_cpp",
        "unowned_equipment_set_use_handler_like_cpp",
    );
    assert_rejected(
        &missing_fixture,
        "missing fixture EquipmentSetUse registrar",
    );

    let mut aliased = actual_mounts();
    aliased[0].source = aliased[0]
        .source
        .replace(
            "wow_world_application::register_equipment_set_use_handler_like_cpp",
            "application_alias::register_equipment_set_use_handler_like_cpp",
        )
        .replace(
            "use std::sync::Arc;",
            "use std::sync::Arc;\nuse wow_world_application as application_alias;",
        );
    assert_rejected(&aliased, "aliased Application EquipmentSetUse registrar");

    let mut wrong_owner = actual_mounts();
    wrong_owner[0].source = wrong_owner[0].source.replace(
        "wow_world_application::register_equipment_set_use_handler_like_cpp",
        "wow_world_application::register_instance_handlers_like_cpp",
    );
    assert_rejected(
        &wrong_owner,
        "EquipmentSetUse call substituted with another registrar",
    );

    let mut missing_facade = actual_mounts();
    let application_root = missing_facade
        .iter_mut()
        .find(|mount| {
            mount.package == EQUIPMENT_SET_USE_REGISTRAR.package
                && mount
                    .source_path
                    .ends_with("crates/wow-world-application/src/lib.rs")
        })
        .expect("Application root source mount exists");
    let missing_export = "register_equipment_set_use_handler_like_cpp,";
    assert_eq!(application_root.source.matches(missing_export).count(), 1);
    let original_root = application_root.source.clone();
    application_root.source = application_root.source.replace(missing_export, "");
    assert_ne!(application_root.source, original_root);
    println!(
        "MISSING FACADE OUTCOME: {:?}",
        validate_composition_mounts(&missing_facade)
    );
    assert_rejected(
        &missing_facade,
        "missing EquipmentSetUse root facade export",
    );

    let mut aliased_facade = actual_mounts();
    let application_root = aliased_facade
        .iter_mut()
        .find(|mount| {
            mount.package == EQUIPMENT_SET_USE_REGISTRAR.package
                && mount
                    .source_path
                    .ends_with("crates/wow-world-application/src/lib.rs")
        })
        .expect("Application root source mount exists");
    let original_root = application_root.source.clone();
    application_root.source = application_root.source.replace(
        "register_equipment_set_use_handler_like_cpp,",
        "register_equipment_set_use_handler_like_cpp as register_use_handler,",
    );
    assert_ne!(application_root.source, original_root);
    assert_rejected(
        &aliased_facade,
        "aliased EquipmentSetUse root facade export",
    );
}

#[test]
fn composition_guard_rejects_missing_duplicate_nested_and_aliased_domain_calls() {
    let mut missing_legacy = actual_mounts();
    missing_legacy[0].source = missing_legacy[0].source.replace(
        "    register_remaining_handlers_like_cpp(&mut builder)?;\n",
        "",
    );
    assert_rejected(&missing_legacy, "missing legacy composer call");

    let mut duplicate = actual_mounts();
    duplicate[0].source = duplicate[0].source.replace(
        "    register_remaining_handlers_like_cpp(&mut builder)?;",
        "    wow_world_inventory::register_inventory_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(&mut builder)?;\n    register_remaining_handlers_like_cpp(&mut builder)?;",
    );
    assert_rejected(&duplicate, "duplicate domain registration");

    let call = "    wow_world_inventory::register_inventory_handlers_like_cpp::<\n        WorldSession,\n        SessionHandlerCatalogsLikeCpp,\n    >(&mut builder)?;";
    let mut nested = actual_mounts();
    nested[0].source = nested[0].source.replace(
        call,
        "    if true {\n        wow_world_inventory::register_inventory_handlers_like_cpp::<\n            WorldSession,\n            SessionHandlerCatalogsLikeCpp,\n        >(&mut builder)?;\n    }",
    );
    assert_rejected(&nested, "conditional/nested registration");

    let mut aliased = actual_mounts();
    aliased[0].source = aliased[0]
        .source
        .replace(
            "use wow_world::session::registry::{",
            "use wow_world_inventory as inventory_owner;\nuse wow_world::session::registry::{",
        )
        .replace(
            "wow_world_inventory::register_inventory_handlers_like_cpp::<",
            "inventory_owner::register_inventory_handlers_like_cpp::<",
        );
    assert_rejected(&aliased, "alternate registrar alias");

    let mut shadowed_package = actual_mounts();
    shadowed_package[0].source = format!(
        "mod wow_world_application {{}}\n{}",
        shadowed_package[0].source
    );
    assert_rejected(
        &shadowed_package,
        "locally shadowed Application package path",
    );
}

#[test]
fn composition_guard_rejects_wrong_mount_and_noncanonical_legacy_import() {
    let mut wrong_module = actual_mounts();
    wrong_module[0].contexts = BTreeSet::from([SourceMountContext {
        logical_module_path: "crate::handlers::other".to_owned(),
        cfg: Vec::new(),
        production_possible: true,
        test_possible: true,
    }]);
    assert_rejected(&wrong_module, "composer mounted outside its owner");

    let mut aliased_legacy = actual_mounts();
    aliased_legacy[0].source = aliased_legacy[0].source.replace(
        "    register_remaining_handlers_like_cpp,\n",
        "    register_remaining_handlers_like_cpp as register_legacy_handlers,\n",
    );
    assert_rejected(&aliased_legacy, "aliased legacy provider import");
}

#[test]
fn composition_guard_rejects_fixture_gate_changes_and_inexact_facades() {
    let mut ungated_fixture = actual_mounts();
    ungated_fixture[1].source = mutate_fixture(
        &ungated_fixture[1].source,
        "#[cfg(any(test, feature = \"test-fixtures\"))]\n#[must_use]\npub fn build_dispatch_table",
        "#[must_use]\npub fn build_dispatch_table",
    );
    assert_rejected(&ungated_fixture, "missing test-fixtures gate");

    let mut altered_expect = actual_mounts();
    altered_expect[1].source = mutate_fixture(
        &altered_expect[1].source,
        "invalid duplicate packet handler composition",
        "duplicate handler",
    );
    assert_rejected(&altered_expect, "changed fixture expect contract");

    let mut missing_root_facade = actual_mounts();
    // rustfmt wrapped this export onto its own line, so the mutation has to
    // match the formatted layout to remove anything.
    missing_root_facade[2].source = mutate_fixture(
        &missing_root_facade[2].source,
        "    register_inventory_handlers_like_cpp,\n",
        "",
    );
    assert_rejected(&missing_root_facade, "missing root facade export");

    let mut aliased_handler_facade = actual_mounts();
    aliased_handler_facade[3].source = mutate_fixture(
        &aliased_handler_facade[3].source,
        "    register_inventory_handlers_like_cpp,\n",
        "    register_inventory_handlers_like_cpp as register_handlers,\n",
    );
    assert_rejected(&aliased_handler_facade, "aliased handlers facade export");

    let mut extra_handler_glob = actual_mounts();
    extra_handler_glob[3].source = mutate_fixture(
        &extra_handler_glob[3].source,
        "    register_inventory_handlers_like_cpp,\n};",
        "    register_inventory_handlers_like_cpp,\n    *,\n};",
    );
    assert_rejected(&extra_handler_glob, "extra glob in handlers facade");

    let mut extra_handler_alias = actual_mounts();
    extra_handler_alias[3].source = mutate_fixture(
        &extra_handler_alias[3].source,
        "    register_inventory_handlers_like_cpp,\n};",
        "    register_inventory_handlers_like_cpp,\n    EquipmentSetsSaveCxLikeCpp as SaveCx,\n};",
    );
    assert_rejected(&extra_handler_alias, "extra alias in handlers facade");

    let mut missing_instances_root_facade = actual_mounts();
    // The Application root facade lists this export on the same line as the next
    // one, so the mutation has to match the real formatting to remove anything.
    missing_instances_root_facade[5].source = mutate_fixture(
        &missing_instances_root_facade[5].source,
        "    register_instance_handlers_like_cpp, reset_represented_instances_like_cpp,\n",
        "    reset_represented_instances_like_cpp,\n",
    );
    assert_rejected(
        &missing_instances_root_facade,
        "missing Application root facade export",
    );

    let mut aliased_instances_module_facade = actual_mounts();
    aliased_instances_module_facade[6].source = mutate_fixture(
        &aliased_instances_module_facade[6].source,
        "pub use registration::register_instance_handlers_like_cpp;",
        "pub use registration::register_instance_handlers_like_cpp as register_handlers;",
    );
    assert_rejected(
        &aliased_instances_module_facade,
        "aliased Application registration facade",
    );
}

#[test]
fn composition_guard_derives_two_owner_call_set_and_rejects_partial_or_alternate_calls() {
    let contracts = DIRECT_REGISTRAR_CONTRACTS;
    let actual = actual_mounts();
    validate_composition_mounts_with_contracts(&actual, contracts).expect(
        "both source-analyzed finite owners are composed in production and fixture builders",
    );

    let mut missing = actual_mounts();
    missing[0].source = missing[0].source.replace(
        "wow_world_application::register_instance_handlers_like_cpp",
        "wow_world_application::unowned_instance_registration",
    );
    assert!(validate_composition_mounts_with_contracts(&missing, contracts).is_err());

    let mut duplicated = actual_mounts();
    duplicated[0].source = duplicated[0].source.replace(
        "    register_remaining_handlers_like_cpp(&mut builder)?;",
        "    wow_world_inventory::register_inventory_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(&mut builder)?;\n    register_remaining_handlers_like_cpp(&mut builder)?;",
    );
    assert!(validate_composition_mounts_with_contracts(&duplicated, contracts).is_err());

    let mut aliased = actual_mounts();
    aliased[0].source = aliased[0]
        .source
        .replace(
            "wow_world_application::register_instance_handlers_like_cpp",
            "application_alias::register_instance_handlers_like_cpp",
        )
        .replace(
            "use wow_world::session::registry::{",
            "use wow_world_application as application_alias;\nuse wow_world::session::registry::{",
        );
    assert!(validate_composition_mounts_with_contracts(&aliased, contracts).is_err());

    let mut outside_composer = actual_mounts();
    outside_composer.push(mount(
        "wow-world-application",
        "crate::instances::other",
        "crates/wow-world-application/src/instances/other.rs",
        "fn extra() { let _ = wow_world_application::register_instance_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>; }",
    ));
    assert!(validate_composition_mounts_with_contracts(&outside_composer, contracts).is_err());

    let mut wrong_type_arguments = actual_mounts();
    wrong_type_arguments[0].source = wrong_type_arguments[0]
        .source
        .replace("WorldSession,", "OtherSession,");
    assert!(validate_composition_mounts_with_contracts(&wrong_type_arguments, contracts).is_err());

    let mut conditional_owner_mount = actual_mounts();
    let owner_source = conditional_owner_mount
        .iter_mut()
        .find(|mount| {
            mount.package == "wow-world-application"
                && mount.source_path.ends_with("instances/registration.rs")
        })
        .expect("Application Instances registrar mount exists");
    owner_source.contexts = BTreeSet::from([SourceMountContext {
        logical_module_path: "crate::instances::registration".to_owned(),
        cfg: vec!["test".to_owned()],
        production_possible: true,
        test_possible: true,
    }]);
    assert!(
        validate_composition_mounts_with_contracts(&conditional_owner_mount, contracts).is_err()
    );

    let mut conditional_root = actual_mounts();
    let app_root = conditional_root
        .iter_mut()
        .find(|mount| {
            mount.package == "wow-world-application" && mount.source_path.ends_with("lib.rs")
        })
        .expect("Application root source mount exists");
    app_root.contexts = BTreeSet::from([SourceMountContext {
        logical_module_path: "crate".to_owned(),
        cfg: vec!["feature = application-fixtures".to_owned()],
        production_possible: true,
        test_possible: true,
    }]);
    assert!(validate_composition_mounts_with_contracts(&conditional_root, contracts).is_err());

    let mut missing_registrar_source = actual_mounts();
    missing_registrar_source.retain(|mount| {
        !(mount.package == "wow-world-application"
            && mount.source_path.ends_with("instances/registration.rs"))
    });
    assert!(
        validate_composition_mounts_with_contracts(&missing_registrar_source, contracts).is_err()
    );

    let extra_owner_facades: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
        module: "crate",
        child: "handlers",
        exports: &["register_unconfigured_handlers_like_cpp"],
    }];
    let extra_owner = DirectRegistrarContract {
        owner: "SyntheticExtraOwner",
        package: "synthetic-extra-owner",
        module: "crate::handlers",
        registrar: "register_unconfigured_handlers_like_cpp",
        host_trait: "SyntheticExtraHostLikeCpp",
        production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
        facades: extra_owner_facades,
    };
    let mut extra_configured_owner = actual_mounts();
    extra_configured_owner.push(mount(
        "synthetic-extra-owner",
        "crate",
        "synthetic-extra-owner/src/lib.rs",
        "pub use handlers::register_unconfigured_handlers_like_cpp;",
    ));
    assert!(
        validate_composition_mounts_with_contracts(
            &extra_configured_owner,
            &[INVENTORY_REGISTRAR, INSTANCES_REGISTRAR, extra_owner],
        )
        .is_err()
    );

    assert!(
        validate_composition_mounts_with_contracts(&actual, &[INVENTORY_REGISTRAR],).is_err(),
        "a configured set missing an actual owner is rejected"
    );
}
