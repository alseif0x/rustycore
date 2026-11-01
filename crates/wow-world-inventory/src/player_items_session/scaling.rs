// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The area-based item-level scaling application, built from the World
//! session's disjoint fields (#1363 slice 12b1b; moved verbatim from the World
//! `session/player_items/valuation.rs`).

use wow_world_core::session::HubMut;

use super::{
    PlayerItemsSessionFieldsLikeCpp, PlayerItemsSessionHostLikeCpp, world_test_consumer_like_cpp,
};

pub fn update_represented_item_level_area_based_scaling_with_publication_like_cpp<
    H: PlayerItemsSessionHostLikeCpp,
>(
    host: &mut H,
    publish: bool,
) -> Option<bool> {
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
    #[cfg(any(test, feature = "test-fixtures"))]
    let player = core.player_stats_access_with_fixture_refs_like_cpp(
        catalogs,
        config,
        &fixtures.identity.player_race,
        &fixtures.identity.player_class,
        &fixtures.identity.player_level,
        wow_world_core::session::StatsFixtureRefs::new_like_cpp(
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut fixtures.combat.player_health_like_cpp,
                &mut fixtures.combat.player_max_health_like_cpp,
                &mut fixtures.combat.player_alive_like_cpp,
                &mut fixtures.combat.represented_player_powers_like_cpp[0],
                &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut fixtures.combat.represented_player_base_mana_like_cpp,
            ),
            wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                &fixtures.auras.represented_shapeshift_form_like_cpp,
                &fixtures.auras.player_aura_authority_complete_like_cpp,
                &fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &fixtures.auras.visible_auras,
                &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            ),
        ),
    );
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let player = core.player_stats_access_like_cpp(catalogs, config);
    crate::InventoryScalingApplicationCxLikeCpp::new(
        inventory,
        player,
        core.owned_inventory_access_like_cpp(),
        core.inventory_valuation_access_like_cpp(),
        core.owned_item_modifiers_access_like_cpp(),
        core.packet_publication_access_like_cpp(),
        catalogs.map_store().map(AsRef::as_ref),
        crate::ItemModsCatalogsViewLikeCpp::new(
            catalogs.items.store.as_ref(),
            catalogs.items.stats_store.as_ref(),
            catalogs.scaling_stat_distribution_store.as_ref(),
            catalogs.scaling_stat_values_store.as_ref(),
            catalogs.shield_block_regular_game_table.as_ref(),
            catalogs.spell_catalogs.spell_shapeshift_form_store(),
        ),
        loot,
        world_test,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.auras.represented_shapeshift_form_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.movement.player_position,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.player_transport_login_state_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        crate::PlayerRegistryHydrationContext::new(
            core.player_registry_hydration_access_like_cpp(),
            spell_state,
            quest_state,
            (
                &fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &fixtures.pets.represented_pet_guid_like_cpp,
            ),
            world_test,
        ),
    )
    .update_item_level_area_based_scaling_like_cpp(publish)
}
