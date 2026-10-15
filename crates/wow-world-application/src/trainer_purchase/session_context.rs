// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The trainer purchase, admission and list contexts, built from the World
//! session's disjoint fields (#1363 slice 12a; moved verbatim from the World
//! `session/trainer_acquisition.rs`). The session lends the fields through
//! [`TrainerSessionHostLikeCpp::split_trainer_mut`]; the former World
//! `cfg!(test)` argument is [`TrainerSessionHostLikeCpp::world_test_consumer_like_cpp`].

use std::sync::Arc;

use wow_core::ObjectGuidGenerator;
use wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp;
use wow_world_core::session::HubMut;
use wow_world_player_conditions::PlayerConditionProjectionInputsLikeCpp;

use super::{
    AppTrainerBuyCx, AppTrainerCx, AppTrainerListCx, TrainerAcquisitionCatalogsLikeCpp,
    TrainerListCatalogsLikeCpp, TrainerSessionFieldsLikeCpp, TrainerSessionHostLikeCpp,
};

/// Borrow the existing disjoint owners and selected catalogs for one
/// trainer operation. Fixture mode follows this World consumer's
/// `cfg(test)` status, even when the shared fixture feature is enabled.
pub(crate) fn trainer_buy_context_like_cpp<'a, H: TrainerSessionHostLikeCpp>(
    host: &'a mut H,
    item_guid_generator: &'a ObjectGuidGenerator,
    battle_pet_selection: &'a BattlePetSelectionStoreLikeCpp,
) -> AppTrainerBuyCx<'a> {
    let world_test = host.world_test_consumer_like_cpp();
    let TrainerSessionFieldsLikeCpp {
        hub:
            HubMut {
                core,
                catalogs: session_catalogs,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                ..
            },
        lifecycle,
        inventory,
        spell_state,
        quest_state,
        loot,
        interaction,
        ..
    } = host.split_trainer_mut();
    let catalogs = TrainerAcquisitionCatalogsLikeCpp::new(
        session_catalogs.skill_store(),
        session_catalogs.skill_line_store(),
        session_catalogs.skill_tiers_store(),
        session_catalogs.item_store(),
        session_catalogs.item_stats_store(),
    );
    let owner = core.player_acquisition_owner_access_like_cpp();

    let acquisition = AppTrainerCx::new(
        owner,
        lifecycle,
        inventory,
        spell_state,
        quest_state,
        loot,
        catalogs,
        world_test,
        #[cfg(any(test, feature = "test-fixtures"))]
        crate::TrainerAcquisitionFixturesLikeCpp::new(
            &fixtures.identity.player_race,
            &fixtures.identity.player_class,
            &fixtures.identity.player_level,
            &mut fixtures.progression.player_skill_test_fixture_like_cpp,
            &mut fixtures.progression.represented_enchanting_skill,
            &fixtures.movement.player_position,
            &fixtures.combat.player_health_like_cpp,
            &fixtures.combat.player_max_health_like_cpp,
            &fixtures.combat.player_alive_like_cpp,
            &fixtures.identity.player_level,
            &fixtures.vehicles.player_transport_login_state_like_cpp,
            &fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            &fixtures.pets.represented_pet_guid_like_cpp,
        ),
    );
    AppTrainerBuyCx::new(
        acquisition,
        interaction,
        session_catalogs
            .trainer_store_like_cpp()
            .map(|store| store.as_ref()),
        battle_pet_selection,
        item_guid_generator,
    )
}

pub(crate) fn trainer_buy_admission_context_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
) -> crate::AppTrainerBuyAdmissionCxLikeCpp<'_> {
    let world_test = host.world_test_consumer_like_cpp();
    let TrainerSessionFieldsLikeCpp {
        hub:
            HubMut {
                core,
                catalogs: session_catalogs,
                config,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                ..
            },
        inventory,
        spell_state,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
        loot,
        interaction,
        ..
    } = host.split_trainer_mut();
    let npc = core.aura_npc_access_builder_like_cpp(
        session_catalogs.factions.store.as_deref(),
        session_catalogs.factions.template_store.as_deref(),
        session_catalogs.friendship_rep_reaction_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.movement.player_position,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_faction_template_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.progression.reputation_state_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_destinations_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_flight_state_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_unit_flags_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_mounted_like_cpp,
    );
    let presentation = core.player_aura_removal_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::AuraRemovalFixtureRefsLikeCpp::new(
            &mut fixtures.auras.player_aura_authority_complete_like_cpp,
            &mut fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp,
            &mut fixtures.auras.visible_auras,
            &mut fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            &mut fixtures.vehicles.player_mount_display_id_like_cpp,
            &mut fixtures.vehicles.player_mounted_like_cpp,
            &mut fixtures.presentation.player_unit_flags_like_cpp,
            &fixtures.presentation.player_object_scale_like_cpp,
        ),
    );
    let control = core.aura_mount_control_access_like_cpp(
        session_catalogs.creatures.display_info_store.as_deref(),
        session_catalogs.creatures.model_data_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::AuraMountControlFixtureRefsLikeCpp::new(
            &mut fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            &mut fixtures.movement.movement_counter_like_cpp,
            &mut fixtures.movement.player_collision_height_like_cpp,
            &fixtures.movement.player_position,
            &mut fixtures.movement.player_movement_flags_like_cpp,
            &fixtures.movement.player_movement_time_like_cpp,
            &mut fixtures
                .movement
                .represented_can_swim_to_fly_transition_like_cpp,
            &fixtures.identity.player_scale_duration_like_cpp,
            &fixtures.identity.player_race,
            &fixtures.identity.player_gender,
            wow_world_core::session::AuraDismountPetFixtureRefsLikeCpp::new(
                &fixtures.pets.represented_pet_guid_like_cpp,
                &mut fixtures.pets.represented_pet_react_state_like_cpp,
                &mut fixtures.pets.represented_pet_command_state_like_cpp,
                &mut fixtures.pets.represented_pet_stable_like_cpp,
                &mut fixtures
                    .pets
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                &mut fixtures
                    .pets
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                &mut fixtures.pets.represented_old_pet_spell_like_cpp,
                &mut fixtures.pets.temporary_mount_pet_react_state_like_cpp,
            ),
            &mut fixtures.movement.movement_speed_rates_like_cpp,
            &mut fixtures.movement.forced_speed_changes_like_cpp,
            &fixtures.pets.represented_pet_guid_like_cpp,
            &mut fixtures.pets.represented_pet_movement_speed_rates_like_cpp,
            &mut fixtures.pets.represented_pet_speed_propagations_like_cpp,
            &fixtures.combat.in_combat,
            &mut fixtures.movement.last_fall_time_like_cpp,
            &mut fixtures.movement.last_fall_z_like_cpp,
        ),
    );
    let stats = core.aura_stats_access_builder_like_cpp(
        session_catalogs,
        config,
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
            &mut fixtures.combat.player_health_like_cpp,
            &mut fixtures.combat.player_max_health_like_cpp,
            &mut fixtures.combat.player_alive_like_cpp,
            &mut fixtures.combat.represented_player_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_base_mana_like_cpp,
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
    );
    let item_sets = core.owned_item_set_access_like_cpp(
        session_catalogs.items.set_store.as_deref(),
        session_catalogs
            .spell_catalogs
            .item_set_spell_store
            .as_deref(),
        session_catalogs.spell_catalogs.spell_store.as_deref(),
        session_catalogs.heirloom_store.as_deref(),
        session_catalogs.items.stats_store.as_deref(),
        session_catalogs.curve_store.as_deref(),
        session_catalogs.curve_point_store.as_deref(),
        session_catalogs.content_tuning_store.as_deref(),
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
    let aura_catalogs = crate::AuraApplicationCatalogsLikeCpp::new(
        session_catalogs.mount_capability_store.as_deref(),
        session_catalogs.spell_catalogs.spell_store.as_ref(),
        session_catalogs.chr.classes_store.as_deref(),
        session_catalogs.difficulty_store().map(AsRef::as_ref),
        session_catalogs.items.store.as_ref(),
        session_catalogs.items.stats_store.as_ref(),
        session_catalogs.items.effect_store.as_ref(),
        session_catalogs
            .spell_catalogs
            .spell_shapeshift_form_store
            .as_deref(),
        session_catalogs.map_store().map(AsRef::as_ref),
        wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
            session_catalogs.items.store.as_ref(),
            session_catalogs.items.stats_store.as_ref(),
            session_catalogs.scaling_stat_distribution_store.as_ref(),
            session_catalogs.scaling_stat_values_store.as_ref(),
            session_catalogs.shield_block_regular_game_table.as_ref(),
            session_catalogs
                .spell_catalogs
                .spell_shapeshift_form_store(),
        ),
    );
    let aura = crate::PlayerAuraApplicationCxLikeCpp::new(
        spell_state,
        inventory,
        presentation,
        control,
        stats,
        item_sets,
        aura_catalogs,
        loot,
        world_test,
        #[cfg(any(test, feature = "test-fixtures"))]
        crate::AuraApplicationFixtureRefsLikeCpp::new(
            &mut fixtures.auras.represented_shapeshift_form_like_cpp,
            &fixtures.identity.player_class,
            &fixtures.identity.player_level,
            quest_state,
            &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            &fixtures.vehicles.player_transport_login_state_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_id_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_accessories_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_seat_count_like_cpp,
            &mut fixtures
                .vehicles
                .player_mount_vehicle_usable_seat_count_like_cpp,
            &mut fixtures.vehicles.mount_vehicle_remove_requests_like_cpp,
            &mut fixtures.pets.mount_pet_control_enable_requests_like_cpp,
            &mut fixtures.pets.mount_pet_resummon_requests_like_cpp,
            &mut fixtures
                .vehicles
                .mount_collision_height_update_requests_like_cpp,
        ),
    );
    crate::AppTrainerBuyAdmissionCxLikeCpp::new(
        aura,
        npc,
        interaction,
        core.trainer_interaction_role_access_like_cpp(),
        session_catalogs.trainer_store_like_cpp().map(Arc::as_ref),
        core.packet_publication_access_like_cpp(),
        core.account_id,
    )
}

pub(crate) fn trainer_list_publication_context_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
) -> AppTrainerListCx<'_> {
    let world_test = host.world_test_consumer_like_cpp();
    let TrainerSessionFieldsLikeCpp {
        hub:
            HubMut {
                core,
                catalogs: session_catalogs,
                config,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                ..
            },
        inventory,
        spell_state,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
        loot,
        interaction,
        social,
        #[cfg(any(test, feature = "test-fixtures"))]
        instances,
        ..
    } = host.split_trainer_mut();
    // Select disjoint field borrows directly: neither inert view borrows
    // the World aggregate that also owns the mutable interaction state.
    let npc = core.aura_npc_access_builder_like_cpp(
        session_catalogs.factions.store.as_deref(),
        session_catalogs.factions.template_store.as_deref(),
        session_catalogs.friendship_rep_reaction_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.movement.player_position,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_faction_template_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.progression.reputation_state_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_destinations_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_flight_state_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_unit_flags_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.vehicles.taxi_mounted_like_cpp,
    );
    let role = core.trainer_interaction_role_access_like_cpp();
    let packets = core.packet_publication_access_like_cpp();
    let spell_access = core.owned_spell_acquisition_access_like_cpp();
    let player_conditions = PlayerConditionProjectionInputsLikeCpp::new(
        core.aura_condition_access_builder_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_gender,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures
                .progression
                .represented_primary_specialization_id_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.movement.player_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_zone_id_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.player_area_id_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures
                .identity
                .player_zone_area_authority_complete_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.combat.player_pvp_hostile_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.combat.player_pvp_end_timer_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.combat.player_contested_pvp_timer_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.identity.represented_is_outdoors_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.vehicles.taxi_destinations_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.vehicles.taxi_flight_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.vehicles.taxi_unit_flags_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures.vehicles.taxi_mounted_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
        ),
        social,
        session_catalogs.chr_specialization_store().map(Arc::as_ref),
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
        #[cfg(any(test, feature = "test-fixtures"))]
        instances,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.battleground,
        session_catalogs.inventory_valuation_catalog_view_like_cpp(),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.progression.reputation_state_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .battleground
            .represented_battleground_status_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_complete_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.combat.in_combat,
        #[cfg(any(test, feature = "test-fixtures"))]
        world_test,
    );
    let catalogs = TrainerListCatalogsLikeCpp::new(
        session_catalogs.trainer_store_like_cpp(),
        session_catalogs
            .spell_catalogs
            .spell_acquisition_catalog()
            .map(Arc::as_ref),
        session_catalogs.skill_store().map(Arc::as_ref),
        session_catalogs.skill_line_store.as_deref(),
        session_catalogs.condition_store().map(Arc::as_ref),
        session_catalogs.player_condition_store().map(Arc::as_ref),
        crate::TrainerProjectionCatalogsLikeCpp::new(
            session_catalogs
                .spell_catalogs
                .spell_chain_store()
                .map(Arc::as_ref),
            session_catalogs
                .spell_catalogs
                .spell_learn_skill_store_like_cpp()
                .map(Arc::as_ref),
            session_catalogs
                .spell_catalogs
                .spell_learn_spell_store_like_cpp()
                .map(Arc::as_ref),
            session_catalogs
                .spell_catalogs
                .spell_required_store_like_cpp()
                .map(Arc::as_ref),
            session_catalogs
                .spell_catalogs
                .spell_custom_attribute_store_like_cpp()
                .map(Arc::as_ref),
            session_catalogs.trait_definition_store().map(Arc::as_ref),
            session_catalogs.skill_tiers_store().map(Arc::as_ref),
            session_catalogs.mount_store().map(Arc::as_ref),
            session_catalogs.difficulty_store().map(Arc::as_ref),
            session_catalogs.map_store().map(Arc::as_ref),
            session_catalogs.disable_mgr().map(Arc::as_ref),
            session_catalogs
                .spell_catalogs
                .spell_target_restrictions_store()
                .map(Arc::as_ref),
            session_catalogs
                .spell_catalogs
                .spell_aura_restrictions_store()
                .map(Arc::as_ref),
            session_catalogs.spell_pet_aura_store_like_cpp(),
            session_catalogs
                .spell_catalogs
                .spell_linked_store_like_cpp(),
        ),
    );
    let presentation = core.player_aura_removal_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::AuraRemovalFixtureRefsLikeCpp::new(
            &mut fixtures.auras.player_aura_authority_complete_like_cpp,
            &mut fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp,
            &mut fixtures.auras.visible_auras,
            &mut fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            &mut fixtures.vehicles.player_mount_display_id_like_cpp,
            &mut fixtures.vehicles.player_mounted_like_cpp,
            &mut fixtures.presentation.player_unit_flags_like_cpp,
            &fixtures.presentation.player_object_scale_like_cpp,
        ),
    );
    let control = core.aura_mount_control_access_like_cpp(
        session_catalogs.creatures.display_info_store.as_deref(),
        session_catalogs.creatures.model_data_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::AuraMountControlFixtureRefsLikeCpp::new(
            &mut fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            &mut fixtures.movement.movement_counter_like_cpp,
            &mut fixtures.movement.player_collision_height_like_cpp,
            &fixtures.movement.player_position,
            &mut fixtures.movement.player_movement_flags_like_cpp,
            &fixtures.movement.player_movement_time_like_cpp,
            &mut fixtures
                .movement
                .represented_can_swim_to_fly_transition_like_cpp,
            &fixtures.identity.player_scale_duration_like_cpp,
            &fixtures.identity.player_race,
            &fixtures.identity.player_gender,
            wow_world_core::session::AuraDismountPetFixtureRefsLikeCpp::new(
                &fixtures.pets.represented_pet_guid_like_cpp,
                &mut fixtures.pets.represented_pet_react_state_like_cpp,
                &mut fixtures.pets.represented_pet_command_state_like_cpp,
                &mut fixtures.pets.represented_pet_stable_like_cpp,
                &mut fixtures
                    .pets
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                &mut fixtures
                    .pets
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                &mut fixtures.pets.represented_old_pet_spell_like_cpp,
                &mut fixtures.pets.temporary_mount_pet_react_state_like_cpp,
            ),
            &mut fixtures.movement.movement_speed_rates_like_cpp,
            &mut fixtures.movement.forced_speed_changes_like_cpp,
            &fixtures.pets.represented_pet_guid_like_cpp,
            &mut fixtures.pets.represented_pet_movement_speed_rates_like_cpp,
            &mut fixtures.pets.represented_pet_speed_propagations_like_cpp,
            &fixtures.combat.in_combat,
            &mut fixtures.movement.last_fall_time_like_cpp,
            &mut fixtures.movement.last_fall_z_like_cpp,
        ),
    );
    let stats = core.aura_stats_access_builder_like_cpp(
        session_catalogs,
        config,
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
            &mut fixtures.combat.player_health_like_cpp,
            &mut fixtures.combat.player_max_health_like_cpp,
            &mut fixtures.combat.player_alive_like_cpp,
            &mut fixtures.combat.represented_player_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_base_mana_like_cpp,
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
    );
    let item_sets = core.owned_item_set_access_like_cpp(
        session_catalogs.items.set_store.as_deref(),
        session_catalogs
            .spell_catalogs
            .item_set_spell_store
            .as_deref(),
        session_catalogs.spell_catalogs.spell_store.as_deref(),
        session_catalogs.heirloom_store.as_deref(),
        session_catalogs.items.stats_store.as_deref(),
        session_catalogs.curve_store.as_deref(),
        session_catalogs.curve_point_store.as_deref(),
        session_catalogs.content_tuning_store.as_deref(),
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
    let aura_catalogs = crate::AuraApplicationCatalogsLikeCpp::new(
        session_catalogs.mount_capability_store.as_deref(),
        session_catalogs.spell_catalogs.spell_store.as_ref(),
        session_catalogs.chr.classes_store.as_deref(),
        session_catalogs.difficulty_store().map(AsRef::as_ref),
        session_catalogs.items.store.as_ref(),
        session_catalogs.items.stats_store.as_ref(),
        session_catalogs.items.effect_store.as_ref(),
        session_catalogs
            .spell_catalogs
            .spell_shapeshift_form_store
            .as_deref(),
        session_catalogs.map_store().map(AsRef::as_ref),
        wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
            session_catalogs.items.store.as_ref(),
            session_catalogs.items.stats_store.as_ref(),
            session_catalogs.scaling_stat_distribution_store.as_ref(),
            session_catalogs.scaling_stat_values_store.as_ref(),
            session_catalogs.shield_block_regular_game_table.as_ref(),
            session_catalogs
                .spell_catalogs
                .spell_shapeshift_form_store(),
        ),
    );
    let aura = crate::PlayerAuraApplicationCxLikeCpp::new(
        spell_state,
        inventory,
        presentation,
        control,
        stats,
        item_sets,
        aura_catalogs,
        loot,
        world_test,
        #[cfg(any(test, feature = "test-fixtures"))]
        crate::AuraApplicationFixtureRefsLikeCpp::new(
            &mut fixtures.auras.represented_shapeshift_form_like_cpp,
            &fixtures.identity.player_class,
            &fixtures.identity.player_level,
            quest_state,
            &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            &fixtures.vehicles.player_transport_login_state_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_id_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_accessories_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_seat_count_like_cpp,
            &mut fixtures
                .vehicles
                .player_mount_vehicle_usable_seat_count_like_cpp,
            &mut fixtures.vehicles.mount_vehicle_remove_requests_like_cpp,
            &mut fixtures.pets.mount_pet_control_enable_requests_like_cpp,
            &mut fixtures.pets.mount_pet_resummon_requests_like_cpp,
            &mut fixtures
                .vehicles
                .mount_collision_height_update_requests_like_cpp,
        ),
    );
    let account_id = core.account_id;
    let session_locale_name = core.session_locale_name_like_cpp();
    AppTrainerListCx::new(
        interaction,
        role,
        packets,
        npc,
        spell_access,
        aura,
        player_conditions,
        catalogs,
        account_id,
        session_locale_name,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_complete_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_occupied_slots_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_non_durable_tombstones_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_loaded_like_cpp,
        &config.max_primary_trade_skills_like_cpp,
    )
}
