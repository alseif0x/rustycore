// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The player-item equip, position-publication and swap-effect contexts,
//! built from the World session's disjoint fields, and the operations that
//! run them (#1363 slice 12b1b; moved verbatim from the World
//! `session/player_items/{publication,storage}.rs` and
//! `handlers/character/items/host.rs`).

use wow_constants::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use crate::{TrainerSessionFieldsLikeCpp, TrainerSessionHostLikeCpp};

fn inventory_equip_cx_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
) -> crate::InventoryEquipCxLikeCpp<'_> {
    let world_test = host.world_test_consumer_like_cpp();
    let TrainerSessionFieldsLikeCpp {
        hub:
            HubMut {
                core,
                catalogs,
                config,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                ..
            },
        lifecycle,
        inventory,
        #[cfg(any(test, feature = "test-fixtures"))]
        spell_state,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
        loot,
        ..
    } = host.split_trainer_mut();
    #[cfg(any(test, feature = "test-fixtures"))]
    let stats_player = core.player_stats_access_with_fixture_refs_like_cpp(
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
    let stats_player = core.player_stats_access_like_cpp(catalogs, config);
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
    #[cfg(any(test, feature = "test-fixtures"))]
    let hydration = if world_test {
        Some(wow_world_inventory::PlayerRegistryHydrationContext::new(
            core.player_registry_hydration_access_like_cpp(),
            &*spell_state,
            &*quest_state,
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
    crate::InventoryEquipCxLikeCpp::new(
        inventory,
        &*lifecycle,
        core.owned_inventory_access_like_cpp(),
        core.owned_item_modifiers_access_like_cpp(),
        item_sets,
        core.packet_publication_access_like_cpp(),
        stats_player,
        core.inventory_valuation_access_like_cpp(),
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
        catalogs.items.effect_store.as_ref(),
        catalogs
            .spell_catalogs
            .spell_item_enchantment_store
            .as_deref(),
        wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
            catalogs.items.store.as_ref(),
            catalogs.items.stats_store.as_ref(),
            catalogs.scaling_stat_distribution_store.as_ref(),
            catalogs.scaling_stat_values_store.as_ref(),
            catalogs.shield_block_regular_game_table.as_ref(),
            catalogs.spell_catalogs.spell_shapeshift_form_store(),
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        catalogs.inventory_valuation_catalog_view_like_cpp(),
        loot,
        core.player_registry_sync_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.movement.player_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.vehicles.player_transport_login_state_like_cpp,
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        crate::InventoryEquipFixtureRefsLikeCpp::new(
            &fixtures.identity.player_level,
            &fixtures.auras.represented_shapeshift_form_like_cpp,
            &fixtures.auras.visible_auras,
        ),
        world_test,
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_player_conditions::MIN_ITEM_LEVEL_LIKE_CPP,
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_player_conditions::MAX_ITEM_LEVEL_LIKE_CPP,
        #[cfg(any(test, feature = "test-fixtures"))]
        hydration,
    )
}

fn inventory_position_publication_cx_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
) -> crate::InventoryPositionPublicationCxLikeCpp<'_> {
    let TrainerSessionFieldsLikeCpp {
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
        ..
    } = host.split_trainer_mut();
    #[cfg(any(test, feature = "test-fixtures"))]
    let stats_player = core.player_stats_access_with_fixture_refs_like_cpp(
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
    let stats_player = core.player_stats_access_like_cpp(catalogs, config);
    crate::InventoryPositionPublicationCxLikeCpp::new(
        &*inventory,
        core.owned_inventory_access_like_cpp(),
        core.packet_publication_access_like_cpp(),
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
        stats_player,
    )
}

fn inventory_swap_effects_cx_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
) -> crate::InventorySwapEffectsCxLikeCpp<'_> {
    let world_test = host.world_test_consumer_like_cpp();
    let TrainerSessionFieldsLikeCpp {
        hub:
            HubMut {
                core,
                catalogs,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                ..
            },
        inventory,
        ..
    } = host.split_trainer_mut();
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
    crate::InventorySwapEffectsCxLikeCpp::new(
        inventory,
        core.owned_inventory_access_like_cpp(),
        core.owned_item_modifiers_access_like_cpp(),
        item_sets,
        core.packet_publication_access_like_cpp(),
        wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
            catalogs.items.store.as_ref(),
            catalogs.items.stats_store.as_ref(),
            catalogs.scaling_stat_distribution_store.as_ref(),
            catalogs.scaling_stat_values_store.as_ref(),
            catalogs.shield_block_regular_game_table.as_ref(),
            catalogs.spell_catalogs.spell_shapeshift_form_store(),
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.auras.represented_shapeshift_form_like_cpp,
        world_test,
    )
}

pub async fn execute_inventory_equip_to_empty_raw_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    source_bag: u8,
    source_slot: u8,
    destination: u16,
) {
    inventory_equip_cx_like_cpp(host)
        .execute_inventory_equip_to_empty_raw_like_cpp(source_bag, source_slot, destination)
        .await;
}

pub fn publish_inventory_position_changes_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    positions: &[(u8, u8)],
) {
    inventory_position_publication_cx_like_cpp(host)
        .publish_inventory_position_changes_like_cpp(positions);
}

pub fn apply_inventory_item_remove_side_effects_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item_guid: ObjectGuid,
    cleared_mainhand_enchantments: &[EnchantmentSlot],
) -> bool {
    inventory_swap_effects_cx_like_cpp(host).remove_item_effects_like_cpp(
        bag,
        slot,
        item_guid,
        cleared_mainhand_enchantments,
    )
}

/// C++ `StoreItem`/`BankItem`/`EquipItem` post-placement side effects for
/// a runtime item whose persistence and position have already committed.
pub fn apply_inventory_item_store_side_effects_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item_guid: ObjectGuid,
) -> bool {
    inventory_swap_effects_cx_like_cpp(host).store_item_effects_like_cpp(bag, slot, item_guid)
}
