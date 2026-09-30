//! Source scan for committed-money publication ordering.
//!
//! Mounted by `character_publication_order.rs`; `include_str!` paths below
//! remain relative to this file's location.

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
        include_str!("../../src/handlers/character/mod.rs"),
        include_str!("../../src/handlers/character/creation_support.rs"),
        include_str!("../../src/handlers/character/enumeration_support.rs"),
        include_str!("../../src/handlers/character/login_support.rs"),
        include_str!("../../src/handlers/character/login_transport_support.rs"),
        include_str!("../../src/handlers/character/account.rs"),
        include_str!("../../src/handlers/character/account/registrations.rs"),
        include_str!("../../src/handlers/character/account/registrations/character_setup.rs"),
        include_str!("../../src/handlers/character/account/registrations/session_services.rs"),
        include_str!("../../src/handlers/character/account/registrations/world_queries.rs"),
        include_str!("../../src/handlers/character/account/registrations/world_services.rs"),
        include_str!("../../src/handlers/character/account/registrations/inventory_actions.rs"),
        include_str!("../../src/handlers/character/account/collections.rs"),
        include_str!("../../src/handlers/character/account/enumeration.rs"),
        include_str!("../../src/handlers/character/bank.rs"),
        include_str!("../../src/handlers/character/condition_objects.rs"),
        include_str!("../../src/handlers/character/entry_zone.rs"),
        include_str!("../../src/handlers/character/gossip.rs"),
        include_str!("../../src/handlers/character/gossip/binder.rs"),
        include_str!("../../src/handlers/character/gossip/conditions.rs"),
        include_str!("../../src/handlers/character/gossip/catalog_menu.rs"),
        include_str!("../../src/handlers/character/gossip/selection.rs"),
        include_str!("../../src/handlers/character/item_load_support.rs"),
        include_str!("../../src/handlers/character/items.rs"),
        include_str!("../../src/handlers/character/items/creature_equipment.rs"),
        include_str!("../../src/handlers/character/items/storage_move.rs"),
        include_str!("../../src/handlers/character/items/turnin.rs"),
        include_str!("../../src/handlers/character/items/handlers.rs"),
        include_str!("../../src/handlers/character/items/destruction.rs"),
        include_str!("../../src/handlers/character/items/equipment_sets.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/validation.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/child_redirect.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/child_equipment.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/swap_execution.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/publication.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/item_mutations.rs"),
        include_str!("../../src/handlers/character/items/inventory_moves/real_swap.rs"),
        include_str!("../../src/handlers/character/items/login_load.rs"),
        include_str!("../../src/handlers/character/items/login_load/inventory.rs"),
        include_str!("../../src/handlers/character/lifecycle.rs"),
        include_str!("../../src/handlers/character/pets.rs"),
        include_str!("../../src/handlers/character/query.rs"),
        include_str!("../../src/handlers/character/session_state.rs"),
        include_str!("../../src/handlers/character/spell_rules.rs"),
        include_str!("../../src/handlers/character/stats.rs"),
        include_str!("../../src/handlers/character/stats_queries.rs"),
        include_str!("../../src/handlers/character/stats_update.rs"),
        include_str!("../../src/handlers/character/vendor.rs"),
        include_str!("../../src/handlers/character/vendor/repair.rs"),
        include_str!("../../src/handlers/character/vendor/refund.rs"),
        include_str!("../../src/handlers/character/vendor/catalog_resolution.rs"),
        include_str!("../../src/handlers/character/vendor/list_inventory.rs"),
        include_str!("../../src/handlers/character/vendor/buy.rs"),
        include_str!("../../src/handlers/character/vendor/buy/currency.rs"),
        include_str!("../../src/handlers/character/vendor/buy/item_purchase.rs"),
        include_str!("../../src/handlers/character/vendor/buy/publication.rs"),
        include_str!("../../src/handlers/character/vendor/buyback.rs"),
        include_str!("../../src/handlers/character/vendor/rules.rs"),
        include_str!("../../src/handlers/character/vendor/rules_tests.rs"),
        include_str!("../../src/handlers/character/vendor/sell.rs"),
        include_str!("../../src/handlers/character/vendor_admission.rs"),
        include_str!("../../src/handlers/character/visibility.rs"),
        include_str!("../../src/handlers/character/world_entry.rs"),
        include_str!("../../src/handlers/character/world_entry/initial_packets.rs"),
        include_str!("../../src/handlers/character/world_entry/login.rs"),
        include_str!("../../src/handlers/character/world_entry/login/admission.rs"),
        include_str!("../../src/handlers/character/world_entry/login/action_buttons.rs"),
        include_str!("../../src/handlers/character/world_entry/login/aura_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/cuf_profiles.rs"),
        include_str!("../../src/handlers/character/world_entry/login/currency_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/default_skills.rs"),
        include_str!("../../src/handlers/character/world_entry/login/glyph_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/group_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/mail_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/pet_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/reputation_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/skill_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/spell_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/spell_map_finalization.rs"),
        include_str!("../../src/handlers/character/world_entry/login/talent_loading.rs"),
        include_str!("../../src/handlers/character/world_entry/login/transport_restore.rs"),
        include_str!("../../src/handlers/character/world_entry/login_recovery.rs"),
    );
    // #236 split the former `session.rs`; the committed-money callers stayed
    // in `mod.rs`. If they move again this scan must follow them - the
    // assertions below fail loudly on a missing marker rather than passing.
    // #597 moved the item and inventory family into `session/player_items`,
    // so the scan follows it there exactly as it followed the #224 character
    // split above.
    let session = concat!(
        include_str!("../../src/session/mod.rs"),
        include_str!("../../src/session/player_items/appearance.rs"),
        include_str!("../../src/session/player_items/appearance/persistence.rs"),
        include_str!("../../src/session/player_items/appearance/catalog.rs"),
        include_str!("../../src/session/player_items/appearance/acquisition.rs"),
        include_str!("../../src/session/player_items/appearance/admission.rs"),
        include_str!("../../src/session/player_items/appearance/transitions.rs"),
        include_str!("../../src/session/player_items/appearance/publication.rs"),
        include_str!("../../src/session/player_items/appearance/criteria.rs"),
        include_str!("../../src/session/player_items/bank.rs"),
        include_str!("../../src/session/player_items/catalog.rs"),
        include_str!("../../src/session/player_items/durability.rs"),
        include_str!("../../src/session/player_items/durability/loss.rs"),
        include_str!("../../src/session/player_items/durability/damage_effects.rs"),
        include_str!("../../src/session/player_items/durability/repair_cost.rs"),
        include_str!("../../src/session/player_items/durability/repair_item.rs"),
        include_str!("../../src/session/player_items/durability/repair_all.rs"),
        include_str!("../../src/session/player_items/enchantment.rs"),
        include_str!("../../src/session/player_items/enchantment/catalog.rs"),
        include_str!("../../src/session/player_items/enchantment/requirements.rs"),
        include_str!("../../src/session/player_items/enchantment/plans.rs"),
        include_str!("../../src/session/player_items/enchantment/runtime_effects.rs"),
        include_str!("../../src/session/player_items/enchantment/duration_persistence.rs"),
        include_str!("../../src/session/player_items/equipment.rs"),
        include_str!("../../src/session/player_items/equipment/equip_spells.rs"),
        include_str!("../../src/session/player_items/equipment/admission.rs"),
        include_str!("../../src/session/player_items/equipment/valuation_and_publication.rs"),
        include_str!("../../src/session/player_items/equipment_sets.rs"),
        include_str!("../../src/session/player_items/equipment_slots.rs"),
        include_str!("../../src/session/player_items/items.rs"),
        include_str!("../../src/session/player_items/items/loot_queries.rs"),
        include_str!("../../src/session/player_items/items/guids.rs"),
        include_str!("../../src/session/player_items/items/item_sets.rs"),
        include_str!("../../src/session/player_items/items/buyback.rs"),
        include_str!("../../src/session/player_items/items/spell_requirements.rs"),
        include_str!("../../src/session/player_items/items/economy_and_pet_adapters.rs"),
        include_str!("../../src/session/player_items/modifiers.rs"),
        include_str!("../../src/session/player_items/modifiers/catalog.rs"),
        include_str!("../../src/session/player_items/modifiers/runtime_access.rs"),
        include_str!("../../src/session/player_items/modifiers/item_sets.rs"),
        include_str!("../../src/session/player_items/modifiers/bonus_planning.rs"),
        include_str!("../../src/session/player_items/modifiers/loaded_replay.rs"),
        include_str!("../../src/session/player_items/modifiers/stat_publication.rs"),
        include_str!("../../src/session/player_items/offhand.rs"),
        include_str!("../../src/session/player_items/persistence.rs"),
        include_str!("../../src/session/player_items/persistence/committed_runtime.rs"),
        include_str!("../../src/session/player_items/persistence/ports.rs"),
        include_str!("../../src/session/player_items/persistence/planning.rs"),
        include_str!("../../src/session/player_items/persistence/durable_fences.rs"),
        include_str!("../../src/session/player_items/persistence/publication.rs"),
        include_str!("../../src/session/player_items/persistence_load.rs"),
        include_str!("../../src/session/player_items/publication.rs"),
        include_str!("../../src/session/player_items/storage.rs"),
        include_str!("../../src/session/player_items/storage/moves.rs"),
        include_str!("../../src/session/player_items/storage/item_objects.rs"),
        include_str!("../../src/session/player_items/storage/owner_access.rs"),
        include_str!("../../src/session/player_items/storage/placement_effects.rs"),
        include_str!("../../src/session/player_items/storage_bags.rs"),
        include_str!("../../src/session/player_items/storage_slots.rs"),
        include_str!("../../src/session/player_items/valuation.rs"),
    );
    assert_publication_segment(
        character,
        "bank-slot purchase",
        "self.set_player_gold_like_cpp(new_money)",
        &[
            "self.set_player_bank_bag_slot_count_like_cpp(new_count)",
            "self.sync_player_registry_state_like_cpp();",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item purchase",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.apply_item_turnin_changes",
            "self.set_player_currencies_like_cpp(planned_currencies)",
            "self.insert_inventory_item_like_cpp",
            "self.update_vendor_item_current_count",
        ],
    );
    assert_publication_segment(
        character,
        "vendor currency purchase",
        "self.set_player_currencies_like_cpp(planned_currencies)",
        &["self.apply_item_turnin_changes"],
    );
    assert_publication_segment(
        character,
        "vendor buyback purchase",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.remove_buyback_item_like_cpp",
            "self.insert_inventory_item_like_cpp",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item sale",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.set_buyback_slot_metadata_like_cpp",
            "self.insert_buyback_item_like_cpp",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item purchase refund",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.remove_inventory_item_like_cpp(refund_slot);",
            "self.insert_inventory_item_like_cpp",
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
