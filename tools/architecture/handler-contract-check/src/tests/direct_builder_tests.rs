// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::path::Path;

use crate::registrations::{
    DirectRegistrarContract, DIRECT_REGISTRAR_CONTRACTS, EQUIPMENT_SET_USE_REGISTRAR,
    INSTANCES_REGISTRAR, INVENTORY_REGISTRAR, analyze_contract_source, analyze_owner_source,
    analyze_owner_source_with_contracts, unowned_entry_literal_violation,
};

const INVENTORY_PACKAGE: &str = "wow-world-inventory";
const INVENTORY_MODULE: &str = "crate::handlers::equipment_sets";

const ONE_ENTRY: &str = r#"
use wow_handler::{DuplicateHandlerRegistrationLikeCpp, PacketHandlerEntry, RegistryBuilder};

pub fn register_inventory_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::OnlyEntry,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "only_entry",
        handler: only_entry,
    })?;
    Ok(())
}
"#;

fn analyze(source: &str) -> Result<crate::registrations::RegistrarReport, String> {
    analyze_owner_source(
        INVENTORY_PACKAGE,
        INVENTORY_MODULE,
        Path::new("equipment_sets.rs"),
        source,
    )
}

#[test]
fn direct_inventory_registrar_accepts_current_source_and_counts_entries() {
    let report = analyze(include_str!("../../../../../crates/wow-world-inventory/src/handlers/equipment_sets.rs"))
        .expect("the production Inventory registrar matches its direct-builder grammar");
    assert_eq!(report.entries, 3);
    assert_eq!(report.registrar_count, 1);
    assert_eq!(analyze(ONE_ENTRY).expect("single direct entry").entries, 1);
    let harmless_text = format!("{ONE_ENTRY}\nconst DOC: &str = \"PacketHandlerEntry RegistryBuilder\";");
    assert_eq!(
        analyze(&harmless_text).expect("text mentioning provider names is not source syntax").entries,
        1
    );
}

#[test]
fn direct_instances_registrar_accepts_exact_seven_entry_source() {
    const SOURCE: &str = include_str!("../../../../../crates/wow-world-application/src/instances/registration.rs");
    let report = analyze_owner_source_with_contracts(
        INSTANCES_REGISTRAR.package,
        INSTANCES_REGISTRAR.module,
        Path::new("registration.rs"),
        SOURCE,
        &[INVENTORY_REGISTRAR, INSTANCES_REGISTRAR],
    )
    .expect("Application Instances has its own exact finite registrar contract");
    assert_eq!(report.entries, 7);
    assert_eq!(report.registrar_count, 1);
    assert_eq!(report.contract, Some(INSTANCES_REGISTRAR));
    assert!(analyze_owner_source_with_contracts(
        INSTANCES_REGISTRAR.package,
        INSTANCES_REGISTRAR.module,
        Path::new("registration.rs"),
        SOURCE,
        &[INVENTORY_REGISTRAR],
    )
    .is_err(), "a direct owner omitted from the finite contract set is rejected");
}

#[test]
fn direct_equipment_set_use_registrar_accepts_its_exact_owner_and_rejects_aliases() {
    const SOURCE: &str = include_str!(
        "../../../../../crates/wow-world-application/src/equipment_set_use.rs"
    );
    let report = analyze_owner_source_with_contracts(
        EQUIPMENT_SET_USE_REGISTRAR.package,
        EQUIPMENT_SET_USE_REGISTRAR.module,
        Path::new("equipment_set_use.rs"),
        SOURCE,
        DIRECT_REGISTRAR_CONTRACTS,
    )
    .expect("Application EquipmentSetUse has its exact finite direct registrar");
    assert_eq!(report.entries, 1);
    assert_eq!(report.registrar_count, 1);
    assert_eq!(report.contract, Some(EQUIPMENT_SET_USE_REGISTRAR));

    let aliased_entry = SOURCE
        .replace("PacketHandlerEntry,", "PacketHandlerEntry as Entry,")
        .replace("builder.register(PacketHandlerEntry", "builder.register(Entry");
    assert!(analyze_owner_source_with_contracts(
        EQUIPMENT_SET_USE_REGISTRAR.package,
        EQUIPMENT_SET_USE_REGISTRAR.module,
        Path::new("equipment_set_use.rs"),
        &aliased_entry,
        DIRECT_REGISTRAR_CONTRACTS,
    )
    .is_err(), "a renamed PacketHandlerEntry binding cannot satisfy the owner contract");

    assert!(analyze_owner_source_with_contracts(
        EQUIPMENT_SET_USE_REGISTRAR.package,
        "crate::equipment_set_use::other",
        Path::new("equipment_set_use.rs"),
        SOURCE,
        DIRECT_REGISTRAR_CONTRACTS,
    )
    .is_err(), "the registrar cannot be moved under a different logical module");

    assert!(analyze_owner_source_with_contracts(
        EQUIPMENT_SET_USE_REGISTRAR.package,
        EQUIPMENT_SET_USE_REGISTRAR.module,
        Path::new("equipment_set_use.rs"),
        SOURCE,
        &[INVENTORY_REGISTRAR, INSTANCES_REGISTRAR],
    )
    .is_err(), "an owner omitted from the finite contract set is rejected");
}

#[test]
fn direct_inventory_registrar_rejects_aliases_control_flow_forwarders_and_wrong_receivers() {
    let aliased = ONE_ENTRY
        .replace("PacketHandlerEntry, RegistryBuilder", "PacketHandlerEntry as Entry, RegistryBuilder")
        .replace("builder.register(PacketHandlerEntry", "builder.register(Entry");
    let conditional = ONE_ENTRY.replace(
        "    builder.register(PacketHandlerEntry {",
        "    if true { builder.register(PacketHandlerEntry {",
    ).replace(
        "        handler: only_entry,\n    })?;",
        "        handler: only_entry,\n    })?; }",
    );
    let nested = ONE_ENTRY.replace(
        "    builder.register(PacketHandlerEntry {",
        "    fn nested() { builder.register(PacketHandlerEntry {",
    ).replace(
        "        handler: only_entry,\n    })?;",
        "        handler: only_entry,\n    })?; }",
    );
    let forwarded = ONE_ENTRY.replace(
        "builder.register(PacketHandlerEntry {\n        opcode: ClientOpcodes::OnlyEntry,\n        status: SessionStatus::LoggedIn,\n        processing: PacketProcessing::Inplace,\n        handler_name: \"only_entry\",\n        handler: only_entry,\n    })?;",
        "register_entries(builder)?;",
    );
    let wrong_receiver = ONE_ENTRY.replace("builder.register(", "other.register(");

    for (name, source) in [
        ("aliased entry", aliased),
        ("conditional registration", conditional),
        ("nested registration", nested),
        ("forwarded registration", forwarded),
        ("wrong receiver", wrong_receiver),
    ] {
        assert!(analyze(&source).is_err(), "accepted {name}");
    }
}

#[test]
fn direct_inventory_registrar_rejects_duplicate_entries_and_nonowner_provider_bindings() {
    let duplicate = ONE_ENTRY.replace(
        "    Ok(())",
        "    builder.register(PacketHandlerEntry {\n        opcode: ClientOpcodes::OnlyEntry,\n        status: SessionStatus::LoggedIn,\n        processing: PacketProcessing::Inplace,\n        handler_name: \"only_entry\",\n        handler: only_entry,\n    })?;\n    Ok(())",
    );
    assert!(analyze(&duplicate).is_err(), "duplicate opcode must fail closed");

    let alias = "type HiddenEntry<S, C> = wow_handler::PacketHandlerEntry<S, C>;";
    assert!(
        unowned_entry_literal_violation(alias)
            .expect("type alias parses")
            .is_none(),
        "a nominal alias without a registration is not itself a submitted entry"
    );
    let alias_literal = "type HiddenEntry<S, C> = wow_handler::PacketHandlerEntry<S, C>; fn outside() { let _ = HiddenEntry {}; }";
    assert!(
        unowned_entry_literal_violation(alias_literal)
            .expect("aliased entry literal parses")
            .is_some(),
        "an aliased entry literal outside the registrar must be reported"
    );
    let imported_alias = "use wow_handler::PacketHandlerEntry as Entry; fn outside() { let _ = Entry {}; }";
    assert!(
        unowned_entry_literal_violation(imported_alias)
            .expect("renamed provider binding parses")
            .is_some(),
        "a renamed provider binding cannot hide an entry literal outside the registrar"
    );
    assert_eq!(
        unowned_entry_literal_violation(
            "type WorldPacketHandlerRegistryBuilder<S, C> = wow_handler::RegistryBuilder<S, C>;"
        )
        .expect("specialized builder alias parses"),
        None,
        "builder aliases without a generic entry are not registrations"
    );
}

#[test]
fn direct_inventory_registrar_rejects_register_calls_outside_its_exact_body() {
    let extra_call = format!(
        "{ONE_ENTRY}\nfn outside<B>(builder: &mut B, entry: impl Into<()>) {{ builder.register(entry); }}"
    );
    assert!(
        analyze(&extra_call).is_err(),
        "a generic register call outside the exact registrar cannot hide an extra route"
    );
}

#[test]
fn nonowner_register_methods_without_packet_entries_are_not_registrations() {
    assert!(
        analyze_owner_source(
            INVENTORY_PACKAGE,
            "crate::handlers::other",
            Path::new("other.rs"),
            ONE_ENTRY,
        )
        .is_err(),
        "the direct Inventory registrar is rejected outside its exact logical module"
    );
    let report = analyze_owner_source(
        "wow-world",
        "crate::handlers::equipment_sets",
        Path::new("unrelated.rs"),
        "fn unrelated(registry: &mut OtherRegistry) { registry.register(item); }",
    )
    .expect("an unrelated method named register does not create handler ownership");
    assert_eq!(report, crate::registrations::RegistrarReport::default());
}

#[test]
fn finite_registrar_contract_binds_function_host_and_exact_owner() {
    const SECOND_OWNER: DirectRegistrarContract = DirectRegistrarContract {
        owner: "FixtureOwner",
        package: "fixture-owner",
        module: "crate::handler_boundary",
        registrar: "register_fixture_handlers_like_cpp",
        host_trait: "FixtureHandlerHostLikeCpp",
        production_type_args: &[],
        facades: &[],
    };
    const SOURCE: &str = r#"
use wow_handler::{DuplicateHandlerRegistrationLikeCpp, PacketHandlerEntry, RegistryBuilder};
pub fn register_fixture_handlers_like_cpp<S, C>(builder: &mut RegistryBuilder<S, C>)
    -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: FixtureHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::FixtureOnly,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "fixture_only",
        handler: fixture_only,
    })?;
    Ok(())
}
"#;
    let accepted = analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        SOURCE,
    )
    .expect("an explicitly supplied finite owner contract is honored");
    assert_eq!(accepted.entries, 1);
    assert_eq!(accepted.registrar_count, 1);
    assert_eq!(accepted.contract, Some(SECOND_OWNER));
    assert_eq!(
        analyze_owner_source_with_contracts(
            "fixture-owner",
            "crate::handler_boundary",
            Path::new("handler_boundary.rs"),
            SOURCE,
            &[SECOND_OWNER],
        )
        .expect("the shared owner analyzer uses the supplied finite contract")
        .contract,
        Some(SECOND_OWNER),
    );

    assert!(analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::other",
        Path::new("other.rs"),
        SOURCE,
    )
    .is_err(), "the same registrar is rejected at a different logical module");
    assert!(analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        &SOURCE.replace("FixtureHandlerHostLikeCpp", "DifferentHostLikeCpp"),
    )
    .is_err(), "a host-trait substitution is rejected");
    assert!(analyze_contract_source(
        SECOND_OWNER,
        "other-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        SOURCE,
    )
    .is_err(), "a registrar cannot be mounted under another package");
    assert!(analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        &SOURCE.replace("pub fn register_fixture_handlers_like_cpp", "#[cfg(test)]\npub fn register_fixture_handlers_like_cpp"),
    )
    .is_err(), "a conditional registrar declaration is rejected");
    assert!(analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        &format!("#![cfg(test)]\n{SOURCE}"),
    )
    .is_err(), "a conditional source module cannot own the registrar");
    assert!(analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        &SOURCE.replace("register_fixture_handlers_like_cpp", "register_unowned_handlers_like_cpp"),
    )
    .is_err(), "entry construction without the named registrar is rejected");
    assert!(analyze_contract_source(
        SECOND_OWNER,
        "fixture-owner",
        "crate::handler_boundary",
        Path::new("handler_boundary.rs"),
        &format!("{SOURCE}\n{SOURCE}"),
    )
    .is_err(), "a duplicate direct registrar declaration is rejected");
    assert_eq!(
        analyze_owner_source(
            INVENTORY_REGISTRAR.package,
            INVENTORY_REGISTRAR.module,
            Path::new("equipment_sets.rs"),
            ONE_ENTRY,
        )
        .expect("Inventory keeps its existing exact contract")
        .entries,
        1,
    );
    assert_eq!(
        analyze_contract_source(
            INVENTORY_REGISTRAR,
            INVENTORY_REGISTRAR.package,
            INVENTORY_REGISTRAR.module,
            Path::new("equipment_sets.rs"),
            ONE_ENTRY,
        )
        .expect("Inventory is still checked against its exact owner contract")
        .contract,
        Some(INVENTORY_REGISTRAR),
    );
}
