// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `#1263 F5 deferred families`: the ten deliberately deferred legacy
//! registrations (void storage 4, talent 2, corpse 4) resolve through their
//! explicit area registrars, and the legacy inventory drain is empty.

use super::*;

/// The void-storage family (`VoidStorageHandler.cpp:33`, `:52`, `:90`, `:206`)
/// left the legacy inventory for the inventory crate's registrar.
#[test]
fn void_storage_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "void storage",
        |builder| {
            wow_world_inventory::register_void_storage_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("void-storage registrar registers");
        },
        &[
            contract_row(
                0x31A2,
                "UnlockVoidStorage",
                "handle_void_storage_unlock",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x31A3,
                "QueryVoidStorage",
                "handle_void_storage_query",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x31A4,
                "VoidStorageTransfer",
                "handle_void_storage_transfer",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x31A5,
                "SwapVoidItem",
                "handle_void_storage_swap_item",
                "LoggedIn",
                "Inplace",
            ),
        ],
        0,
    );
}

/// The talent family (`SkillHandler.cpp:29`, `:43`) left the legacy inventory
/// for the application crate's registrar.
#[test]
fn talent_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "talent",
        |builder| {
            wow_world_application::register_talent_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("talent registrar registers");
        },
        &[
            contract_row(
                0x320D,
                "ConfirmRespecWipe",
                "handle_confirm_respec_wipe",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x3552,
                "LearnTalent",
                "handle_learn_talent",
                "LoggedIn",
                "ThreadUnsafe",
            ),
        ],
        0,
    );
}

/// The death/resurrection/corpse family (`MiscHandler.cpp:447`, `:59`, `:416`,
/// `:361`) left the legacy inventory for the application crate's registrar.
#[test]
fn corpse_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "corpse",
        |builder| {
            wow_world_application::register_corpse_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("corpse registrar registers");
        },
        &[
            contract_row(
                0x3685,
                "ResurrectResponse",
                "handle_resurrect_response",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x3526,
                "RepopRequest",
                "handle_repop_request",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x34DB,
                "ReclaimCorpse",
                "handle_reclaim_corpse",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x3179,
                "RequestCemeteryList",
                "handle_request_cemetery_list",
                "LoggedIn",
                "Inplace",
            ),
        ],
        0,
    );
}

/// F5 is complete: the legacy inventory drain contributes no row, and every one
/// of the 476 composed registrations originates from an explicit area
/// registrar.
#[test]
fn legacy_inventory_drain_is_empty_and_every_registration_is_area_owned() {
    assert!(
        legacy_inventory_contract_rows().is_empty(),
        "no packet handler may still be submitted through the legacy inventory"
    );
    assert_eq!(
        registered_contract().len(),
        476,
        "the composed registry keeps its 476 effective registrations"
    );
}

/// Negative control: each of the three new registrars rejects a second
/// registration of its own first entry and never replaces the original.
#[test]
fn duplicated_deferred_family_registrations_are_rejected_without_replacement() {
    type Registrar = fn(
        &mut crate::session::registry::WorldPacketHandlerRegistryBuilder,
    ) -> Result<(), wow_handler::DuplicateHandlerRegistrationLikeCpp>;

    let registrars: &[(&str, Registrar, wow_constants::ClientOpcodes, &str)] = &[
        (
            "void storage",
            wow_world_inventory::register_void_storage_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >,
            wow_constants::ClientOpcodes::UnlockVoidStorage,
            "handle_void_storage_unlock",
        ),
        (
            "talent",
            wow_world_application::register_talent_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >,
            wow_constants::ClientOpcodes::ConfirmRespecWipe,
            "handle_confirm_respec_wipe",
        ),
        (
            "corpse",
            wow_world_application::register_corpse_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >,
            wow_constants::ClientOpcodes::ResurrectResponse,
            "handle_resurrect_response",
        ),
    ];

    for entry in registrars {
        let (family, register, opcode, handler_name) = *entry;
        let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
        register(&mut builder).expect("the first family registration succeeds");
        let error = register(&mut builder).expect_err("a duplicate registration must be rejected");
        assert_eq!(error.opcode, opcode, "{family}: rejected opcode");
        assert_eq!(
            error.previous_handler_name, handler_name,
            "{family}: the original entry stays in the builder"
        );
        assert_eq!(
            error.new_handler_name, handler_name,
            "{family}: the rejected entry is the duplicate"
        );
        let registry = builder.build();
        assert_eq!(
            registry
                .iter()
                .filter(|candidate| candidate.opcode == opcode)
                .count(),
            1,
            "{family}: the rejected duplicate must not replace the original entry"
        );
    }
}
