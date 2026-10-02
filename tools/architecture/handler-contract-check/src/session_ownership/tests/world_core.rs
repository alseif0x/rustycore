//! P4a scanner coverage for the extracted `wow-world-core` package.

use std::fs;

use super::scenarios_1::{server_source, synthetic_baseline_with_core, world_source};
use super::*;

const NETWORK_WITH_CORE_PAYLOAD: &str =
    "pub enum SessionCommand { Core(crate::session::mailbox::CoreMailboxPayload) }";

fn session_contracts() -> &'static str {
    "pub mod mailbox { pub struct CoreMailboxPayload { pub player: crate::player_directory::PlayerDirectoryPayload, pub session: crate::session::directory::SessionDirectoryPayload } } \
     pub mod directory { pub struct SessionDirectoryPayload { pub loot: crate::loot_persistence::LootPersistencePayload } }"
}

fn root_contracts(loot_amount: &str) -> String {
    format!(
        "pub mod player_directory {{ pub struct PlayerDirectoryPayload {{ pub id: u64 }} }} \
         pub mod loot_persistence {{ pub struct LootPersistencePayload {{ pub amount: {loot_amount} }} }}"
    )
}

fn world_core_contract_source(loot_amount: &str) -> String {
    format!(
        "pub mod session {{ {} }} {}",
        session_contracts(),
        root_contracts(loot_amount),
    )
}

fn legacy_world_contract_source(loot_amount: &str) -> String {
    let mut world = world_source("", "");
    world = world.replacen(
        "pub mod session {",
        &format!("pub mod session {{ {} ", session_contracts()),
        1,
    );
    format!("{world} {}", root_contracts(loot_amount))
}

fn world_with_duplicate_mailbox_payload() -> String {
    world_source("", "").replacen(
        "pub mod session {",
        "pub mod session { pub mod mailbox { pub struct CoreMailboxPayload { pub legacy: u32 } } ",
        1,
    )
}

fn synthetic_core_baseline(world: &str, world_core: &str) -> Result<SessionSyntaxBaseline, String> {
    synthetic_baseline_with_core(
        world,
        &server_source("", ""),
        NETWORK_WITH_CORE_PAYLOAD,
        world_core,
    )
}

fn assert_destination_contracts(baseline: &SessionSyntaxBaseline) {
    for (module, name) in [
        ("crate::session::mailbox", "CoreMailboxPayload"),
        ("crate::player_directory", "PlayerDirectoryPayload"),
        ("crate::session::directory", "SessionDirectoryPayload"),
        ("crate::loot_persistence", "LootPersistencePayload"),
    ] {
        assert!(
            baseline
                .session_command_payload_types
                .iter()
                .any(|surface| {
                    surface.definition.module == module && surface.definition.name == name
                }),
            "missing transitive contract {module}::{name}: {:?}",
            baseline.session_command_payload_types,
        );
    }
}

fn repository_core_units(root: &std::path::Path) -> Result<Vec<SourceUnit>, String> {
    repository_units(
        root,
        PackageRole::WorldCore,
        WORLD_CORE_PACKAGE_ROOT,
        WORLD_CORE_CRATE_ROOT,
    )
}

#[test]
fn world_core_payload_contracts_are_collected_from_destination_modules() {
    let world = world_source("", "");
    let baseline = synthetic_core_baseline(&world, &world_core_contract_source("u32"))
        .expect("core contract fixture parses");
    assert_destination_contracts(&baseline);

    let legacy_world = legacy_world_contract_source("u32");
    let legacy = synthetic_core_baseline(&legacy_world, "")
        .expect("pre-P4a World role contract fixture parses");
    assert_destination_contracts(&legacy);
}

#[test]
fn world_core_duplicate_contract_definitions_are_rejected() {
    let world = world_with_duplicate_mailbox_payload();
    let error = synthetic_core_baseline(&world, &world_core_contract_source("u32"))
        .expect_err("a facade and destination duplicate must be ambiguous");

    assert!(error.contains("CoreMailboxPayload is ambiguous"), "{error}");
}

#[test]
fn world_core_contract_mutations_change_the_pinned_payload_surface() {
    let world = world_source("", "");
    let baseline = synthetic_core_baseline(&world, &world_core_contract_source("u32"))
        .expect("baseline core contract parses");
    let mutated = synthetic_core_baseline(&world, &world_core_contract_source("u64"))
        .expect("mutated core contract parses");

    let error = compare_baseline(&baseline, &mutated)
        .expect_err("a destination payload field change must be visible to the baseline");
    assert!(
        error.contains("obsolete SessionCommand transitive payload type baseline entry"),
        "{error}"
    );
    assert!(
        error.contains("unreviewed SessionCommand transitive payload type surface"),
        "{error}"
    );
}

#[test]
fn world_core_world_session_structs_and_impls_are_rejected() {
    let world = world_source("", "");
    for declaration in [
        "pub struct WorldSession;",
        "impl crate::session::WorldSession {}",
    ] {
        let core = format!("{}\n{declaration}", world_core_contract_source("u32"));
        let error = synthetic_core_baseline(&world, &core)
            .expect_err("WorldSession ownership must stay in wow-world");
        assert!(error.contains("wow-world-core"), "{error}");
        assert!(error.contains("WorldSession"), "{error}");
    }
}

#[test]
fn world_core_repository_root_is_loaded_and_required() {
    let repository_root = std::env::temp_dir().join(format!(
        "handler-contract-world-core-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos(),
    ));
    let crate_root = repository_root.join(WORLD_CORE_CRATE_ROOT);
    fs::create_dir_all(crate_root.parent().expect("core root parent"))
        .expect("create core root parent");
    fs::write(&crate_root, "pub mod session { pub mod mailbox {} }\n").expect("write core root");

    let units =
        repository_core_units(&repository_root).expect("the production core root must be mounted");
    assert_eq!(PackageRole::WorldCore.package_name(), "wow-world-core");
    assert!(units.iter().any(|unit| {
        unit.role == PackageRole::WorldCore
            && unit.repository_relative_path == WORLD_CORE_CRATE_ROOT
            && unit.availability.production
    }));

    fs::remove_file(&crate_root).expect("remove core root for missing-root case");
    let error = repository_core_units(&repository_root)
        .err()
        .expect("a missing production core root must fail loading");
    assert!(error.contains("lib.rs"), "{error}");

    fs::remove_dir_all(repository_root).expect("remove temporary repository");
}
