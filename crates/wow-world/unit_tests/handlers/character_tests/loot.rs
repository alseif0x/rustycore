//! Loot scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

#[test]
fn committed_money_callers_publish_all_runtime_state_before_reopening_admission() {
    fn assert_publication_segment(
        source: &str,
        operation: &str,
        publication_start: &str,
        required_runtime_publications: &[&str],
    ) {
        let operation_marker = format!("\"{operation}\"");
        let operation_offset = source
            .find(&operation_marker)
            .unwrap_or_else(|| panic!("missing operation marker {operation_marker}"));
        let after_operation = &source[operation_offset..];
        let publication_offset = after_operation.find(publication_start).unwrap_or_else(|| {
            panic!("{operation}: missing publication start {publication_start}")
        });
        let after_publication = &after_operation[publication_offset..];
        let drop_offset = after_publication
            .find("drop(money_persistence);")
            .unwrap_or_else(|| panic!("{operation}: missing money guard drop"));
        let publication = &after_publication[..drop_offset];

        assert!(
            !publication.contains(".await"),
            "{operation}: an await reintroduced a post-COMMIT cancellation point before runtime publication"
        );
        for required in required_runtime_publications {
            assert!(
                publication.contains(required),
                "{operation}: runtime publication `{required}` must precede the money guard drop"
            );
        }
    }

    // #224 split the former `character.rs` into private feature modules; this
    // publication-order scan must still see the whole family's source.
    let character = concat!(
        include_str!("../../../src/handlers/character/mod.rs"),
        include_str!("../../../src/handlers/character/creation_support.rs"),
        include_str!("../../../src/handlers/character/enumeration_support.rs"),
        include_str!("../../../src/handlers/character/login_support.rs"),
        include_str!("../../../src/handlers/character/login_transport_support.rs"),
        include_str!("../../../src/handlers/character/account.rs"),
        include_str!("../../../src/handlers/character/account/host.rs"),
        include_str!("../../../src/handlers/character/account/registrations.rs"),
        include_str!("../../../src/handlers/character/account/registrations/character_setup.rs"),
        // #1263 F5: the character/account registration family moved off the
        // legacy inventory path into the application crate's explicit area
        // registrar, so this scan follows it there exactly as it followed the
        // #224 character split above.
        include_str!("../../../../wow-world-application/src/character_account_handlers.rs"),
        include_str!("../../../src/handlers/character/account/collections.rs"),
        include_str!("../../../src/handlers/character/bank.rs"),
        include_str!("../../../src/handlers/character/condition_objects.rs"),
        include_str!("../../../../wow-world-character/src/login_session/entry_zone.rs"),
        include_str!("../../../../wow-world-character/src/gossip_session.rs"),
        include_str!("../../../src/handlers/character/items.rs"),
        include_str!("../../../src/handlers/character/items/handlers.rs"),
        include_str!("../../../src/handlers/character/items/equipment_sets.rs"),
        include_str!("../../../src/handlers/character/items/host.rs"),
        include_str!("../../../../wow-world-character/src/login_session/inventory_load.rs"),
        // #1363 slice 8c: the item handler, inventory move and autobank bodies
        // moved to wow-world-inventory (item_session), so the scan follows them there.
        include_str!("../../../../wow-world-inventory/src/item_session/mod.rs"),
        include_str!("../../../../wow-world-inventory/src/item_session/bank.rs"),
        include_str!("../../../../wow-world-inventory/src/item_session/destruction.rs"),
        include_str!("../../../../wow-world-inventory/src/item_session/handlers.rs"),
        include_str!("../../../../wow-world-inventory/src/item_session/item_mutations.rs"),
        include_str!("../../../../wow-world-inventory/src/item_session/moves.rs"),
        include_str!("../../../../wow-world-inventory/src/item_session/real_swap.rs"),
        include_str!("../../../../wow-world-character/src/login_session/lifecycle.rs"),
        include_str!("../../../src/handlers/character/query.rs"),
        include_str!("../../../../wow-world-character/src/login_session/login_sequence.rs"),
        include_str!("../../../../wow-world-spell/src/login_spell_rules.rs"),
        include_str!("../../../src/handlers/character/stats.rs"),
        include_str!("../../../src/handlers/character/stats_update.rs"),
        include_str!("../../../src/handlers/character/vendor.rs"),
        include_str!("../../../src/handlers/character/vendor/rules.rs"),
        include_str!("../../../../wow-world-inventory/unit_tests/vendor_rules.rs"),
        // #1363 slice 8b: the vendor and bank bodies moved to wow-world-inventory
        // (vendor_session), so the scan follows them there.
        include_str!("../../../../wow-world-inventory/src/vendor_session/mod.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/bank.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/buy.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/buyback.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/catalog.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/list_inventory.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/refund.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/repair.rs"),
        include_str!("../../../../wow-world-inventory/src/vendor_session/sell.rs"),
        include_str!("../../../src/handlers/character/visibility.rs"),
        include_str!("../../../../wow-world-character/src/login_session/world_entry.rs"),
        include_str!("../../../../wow-world-character/src/login_session/initial_packets.rs"),
        include_str!("../../../src/handlers/character/world_entry/login.rs"),
        include_str!("../../../../wow-world-character/src/login_session/admission.rs"),
        include_str!("../../../../wow-world-character/src/login_session/action_buttons.rs"),
        include_str!("../../../../wow-world-character/src/login_session/aura_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/cuf_profiles.rs"),
        include_str!("../../../../wow-world-character/src/login_session/currency_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/default_skills.rs"),
        include_str!("../../../../wow-world-character/src/login_session/glyph_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/group_loading.rs"),
        include_str!("../../../../wow-world-lifecycle/src/pet_session/login_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/reputation_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/skill_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/spell_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/spell_map_finalization.rs"),
        include_str!("../../../../wow-world-character/src/login_session/talent_loading.rs"),
        include_str!("../../../../wow-world-character/src/login_session/transport_restore.rs"),
        include_str!("../../../../wow-world-character/src/login_session/login_recovery.rs"),
    );
    // #236 split the former `session.rs`; the committed-money callers stayed
    // in `mod.rs`. If they move again this scan must follow them - the
    // assertions below fail loudly on a missing marker rather than passing.
    // #597 moved the item and inventory family into `session/player_items`,
    // so the scan follows it there exactly as it followed the #224 character
    // split above.
    let session = concat!(
        include_str!("../../../src/session/mod.rs"),
        include_str!("../../../src/session/player_items/appearance.rs"),
        include_str!("../../../src/session/player_items/bank.rs"),
        include_str!("../../../src/session/player_items/catalog.rs"),
        include_str!("../../../src/session/player_items/durability.rs"),
        include_str!("../../../src/session/player_items/enchantment.rs"),
        include_str!("../../../src/session/player_items/equipment.rs"),
        include_str!("../../../src/session/player_items/equipment_sets.rs"),
        include_str!("../../../../wow-world-inventory/src/equipment_slots.rs"),
        include_str!("../../../src/session/player_items/items.rs"),
        include_str!("../../../src/session/player_items/modifiers.rs"),
        include_str!("../../../src/session/player_items/offhand.rs"),
        include_str!("../../../src/session/player_items/persistence.rs"),
        include_str!("../../../src/session/player_items/persistence_load.rs"),
        include_str!("../../../src/session/player_items/publication.rs"),
        include_str!("../../../src/session/player_items/storage.rs"),
        include_str!("../../../src/session/player_items/storage_bags.rs"),
        include_str!("../../../src/session/player_items/storage_slots.rs"),
        include_str!("../../../src/session/player_items/valuation.rs"),
    );
    assert_publication_segment(
        character,
        "bank-slot purchase",
        "state.set_player_gold_like_cpp(&mut hub, new_money)",
        &[
            "state.set_player_bank_bag_slot_count_like_cpp(&mut hub, new_count)",
            "host.sync_player_registry_state_like_cpp();",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item purchase",
        "host.stage_player_money_change_like_cpp",
        &[
            "host.apply_item_turnin_changes",
            "state.set_player_currencies_like_cpp(&mut hub, planned_currencies)",
            "host.insert_inventory_item_like_cpp",
            "update_vendor_item_current_count(\n            host,",
        ],
    );
    assert_publication_segment(
        character,
        "vendor currency purchase",
        "state.set_player_currencies_like_cpp(&mut hub, planned_currencies)",
        &["host.apply_item_turnin_changes"],
    );
    assert_publication_segment(
        character,
        "vendor buyback purchase",
        "host.stage_player_money_change_like_cpp",
        &[
            "state.remove_buyback_item_like_cpp",
            "host.insert_inventory_item_like_cpp",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item sale",
        "host.stage_player_money_change_like_cpp",
        &[
            "state.set_buyback_slot_metadata_like_cpp",
            "state.insert_buyback_item_like_cpp",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item purchase refund",
        "host.stage_player_money_change_like_cpp",
        &[
            "state.remove_inventory_item_like_cpp(&mut hub, refund_slot)",
            "host.insert_inventory_item_like_cpp",
        ],
    );
    assert_publication_segment(
        session,
        "single item durability repair",
        "self.stage_player_money_change_like_cpp",
        &["self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)"],
    );
    assert_publication_segment(
        session,
        "all-items durability repair",
        "self.stage_player_money_change_like_cpp",
        &["self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)"],
    );
}
