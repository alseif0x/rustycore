// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The equipment-set use context, built from the World session's disjoint
//! fields (#1363 slice 12b1b; moved verbatim from the World
//! `session/player_items/equipment_sets.rs`).

use wow_world_core::session::HubMut;

use super::{
    PlayerItemsSessionFieldsLikeCpp, PlayerItemsSessionHostLikeCpp, world_test_consumer_like_cpp,
};

pub fn build_equipment_set_use_context_like_cpp<H: PlayerItemsSessionHostLikeCpp>(
    host: &mut H,
) -> crate::EquipmentSetUseContextLikeCpp<'_> {
    let world_test = world_test_consumer_like_cpp(host);
    let PlayerItemsSessionFieldsLikeCpp {
        hub:
            HubMut {
                core,
                catalogs,
                config,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                ..
            },
        inventory,
        loot,
        #[cfg(any(test, feature = "test-fixtures"))]
        spell_state,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
    } = host.split_player_items_mut();
    let combat = core.equipment_set_combat_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.combat.in_combat,
    );
    let owner = core.equipment_set_use_access_like_cpp();
    let inventory_access = core.owned_inventory_access_like_cpp();
    let modifier_access = core.owned_item_modifiers_access_like_cpp();
    let item_sets = core.owned_item_set_access_like_cpp(
        catalogs.items.set_store.as_deref(),
        catalogs.spell_catalogs.item_set_spell_store.as_deref(),
        catalogs.spell_catalogs.spell_store.as_deref(),
        catalogs.heirloom_store.as_deref(),
        catalogs.items.stats_store.as_deref(),
        catalogs.curve_store.as_deref(),
        catalogs.curve_point_store.as_deref(),
        catalogs.content_tuning_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .represented_primary_specialization_id_like_cpp,
    );
    let item_mods = crate::EquipmentSetUseItemModsStoresLikeCpp::new(
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
        catalogs.scaling_stat_distribution_store.as_ref(),
        catalogs.scaling_stat_values_store.as_ref(),
        catalogs.shield_block_regular_game_table.as_ref(),
        catalogs.spell_catalogs.spell_shapeshift_form_store(),
    );
    #[cfg(any(test, feature = "test-fixtures"))]
    let registry_hydration = if world_test {
        Some(crate::PlayerRegistryHydrationContext::new(
            core.player_registry_hydration_access_like_cpp(),
            spell_state,
            quest_state,
            (
                &fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &fixtures.pets.represented_pet_guid_like_cpp,
            ),
            true,
        ))
    } else {
        None
    };

    crate::EquipmentSetUseContextLikeCpp::new(
        inventory,
        owner,
        inventory_access,
        modifier_access,
        item_sets,
        combat,
        item_mods,
        catalogs,
        config,
        core.packet_publication_access_like_cpp(),
        loot,
        #[cfg(any(test, feature = "test-fixtures"))]
        crate::EquipmentSetUseFixtureRefsLikeCpp::new(
            &fixtures.movement.player_position,
            &fixtures.identity.player_race,
            &fixtures.identity.player_class,
            &fixtures.identity.player_level,
            &mut fixtures.combat.player_health_like_cpp,
            &mut fixtures.combat.player_max_health_like_cpp,
            &mut fixtures.combat.player_alive_like_cpp,
            &mut fixtures.combat.represented_player_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_base_mana_like_cpp,
            &fixtures.auras.represented_shapeshift_form_like_cpp,
            &fixtures.auras.player_aura_authority_complete_like_cpp,
            &fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp,
            &fixtures.auras.visible_auras,
            &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            &fixtures.vehicles.player_transport_login_state_like_cpp,
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        registry_hydration,
        world_test,
    )
}
