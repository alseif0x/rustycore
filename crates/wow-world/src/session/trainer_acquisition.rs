// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected World-session inputs for the admitted trainer application.

use super::WorldSession;
use std::sync::Arc;
use wow_core::ObjectGuidGenerator;
use wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp;
use wow_world_application::{
    AppTrainerBuyCx, AppTrainerCx, AppTrainerListCx, TrainerAcquisitionCatalogsLikeCpp,
    TrainerListCatalogsLikeCpp,
};
impl WorldSession {
    /// Borrow the existing disjoint owners and selected catalogs for one
    /// trainer operation. Fixture mode follows this World consumer's
    /// `cfg(test)` status, even when the shared fixture feature is enabled.
    pub(crate) fn trainer_buy_context_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        battle_pet_selection: &'a BattlePetSelectionStoreLikeCpp,
    ) -> AppTrainerBuyCx<'a> {
        let catalogs = TrainerAcquisitionCatalogsLikeCpp::new(
            self.catalogs.skill_store(),
            self.catalogs.skill_line_store(),
            self.catalogs.skill_tiers_store(),
            self.catalogs.item_store(),
            self.catalogs.item_stats_store(),
        );
        let owner = self.core.player_acquisition_owner_access_like_cpp();

        let acquisition = AppTrainerCx::new(
            owner,
            &mut self.lifecycle,
            &mut self.inventory,
            &mut self.spell_state,
            &mut self.quest_state,
            &self.loot,
            catalogs,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::TrainerAcquisitionFixturesLikeCpp::new(
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.identity.player_level,
                &mut self.fixtures.progression.player_skill_test_fixture_like_cpp,
                &mut self.fixtures.progression.represented_enchanting_skill,
                &self.fixtures.movement.player_position,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.identity.player_level,
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
                &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
            ),
        );
        AppTrainerBuyCx::new(
            acquisition,
            &mut self.interaction,
            self.catalogs
                .trainer_store_like_cpp()
                .map(|store| store.as_ref()),
            battle_pet_selection,
            item_guid_generator,
        )
    }

    pub(crate) fn trainer_buy_admission_context_like_cpp(
        &mut self,
    ) -> wow_world_application::AppTrainerBuyAdmissionCxLikeCpp<'_> {
        let npc = self.core.aura_npc_access_builder_like_cpp(
            self.catalogs.factions.store.as_deref(),
            self.catalogs.factions.template_store.as_deref(),
            self.catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.movement.player_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_faction_template_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_destinations_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_flight_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_mounted_like_cpp,
        );
        let presentation = self.core.player_aura_removal_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::AuraRemovalFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &mut self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                &mut self.fixtures.auras.visible_auras,
                &mut self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &mut self.fixtures.vehicles.player_mount_display_id_like_cpp,
                &mut self.fixtures.vehicles.player_mounted_like_cpp,
                &mut self.fixtures.presentation.player_unit_flags_like_cpp,
                &self.fixtures.presentation.player_object_scale_like_cpp,
            ),
        );
        let control = self.core.aura_mount_control_access_like_cpp(
            self.catalogs.creatures.display_info_store.as_deref(),
            self.catalogs.creatures.model_data_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::AuraMountControlFixtureRefsLikeCpp::new(
                &mut self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &mut self.fixtures.movement.movement_counter_like_cpp,
                &mut self.fixtures.movement.player_collision_height_like_cpp,
                &self.fixtures.movement.player_position,
                &mut self.fixtures.movement.player_movement_flags_like_cpp,
                &self.fixtures.movement.player_movement_time_like_cpp,
                &mut self.fixtures.movement.represented_can_swim_to_fly_transition_like_cpp,
                &self.fixtures.identity.player_scale_duration_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_gender,
                wow_world_core::session::AuraDismountPetFixtureRefsLikeCpp::new(
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                    &mut self.fixtures.pets.represented_pet_react_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_command_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_stable_like_cpp,
                    &mut self.fixtures.pets.represented_character_pet_rows_empty_authority_complete_like_cpp,
                    &mut self.fixtures.pets.represented_temporary_unsummoned_pet_number_like_cpp,
                    &mut self.fixtures.pets.represented_old_pet_spell_like_cpp,
                    &mut self.fixtures.pets.temporary_mount_pet_react_state_like_cpp,
                ),
                &mut self.fixtures.movement.movement_speed_rates_like_cpp,
                &mut self.fixtures.movement.forced_speed_changes_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
                &mut self.fixtures.pets.represented_pet_movement_speed_rates_like_cpp,
                &mut self.fixtures.pets.represented_pet_speed_propagations_like_cpp,
                &self.fixtures.combat.in_combat,
                &mut self.fixtures.movement.last_fall_time_like_cpp,
                &mut self.fixtures.movement.last_fall_z_like_cpp,
            ),
        );
        let stats = self.core.aura_stats_access_builder_like_cpp(
            &self.catalogs, &self.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut self.fixtures.combat.player_health_like_cpp,
                &mut self.fixtures.combat.player_max_health_like_cpp,
                &mut self.fixtures.combat.player_alive_like_cpp,
                &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
            ),
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_level,
        );
        let item_sets = self.core.owned_item_set_access_like_cpp(
            self.catalogs.items.set_store.as_deref(),
            self.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs.heirloom_store.as_deref(), self.catalogs.items.stats_store.as_deref(),
            self.catalogs.curve_store.as_deref(), self.catalogs.curve_point_store.as_deref(),
            self.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.represented_primary_specialization_id_like_cpp,
        );
        let aura_catalogs = wow_world_application::AuraApplicationCatalogsLikeCpp::new(
            self.catalogs.mount_capability_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_ref(),
            self.catalogs.chr.classes_store.as_deref(), self.catalogs.difficulty_store().map(AsRef::as_ref),
            self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(),
            self.catalogs.items.effect_store.as_ref(),
            self.catalogs.spell_catalogs.spell_shapeshift_form_store.as_deref(),
            self.catalogs.map_store().map(AsRef::as_ref),
            wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
                self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(),
                self.catalogs.scaling_stat_distribution_store.as_ref(),
                self.catalogs.scaling_stat_values_store.as_ref(),
                self.catalogs.shield_block_regular_game_table.as_ref(),
                self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
            ),
        );
        let aura = wow_world_application::PlayerAuraApplicationCxLikeCpp::new(
            &mut self.spell_state, &mut self.inventory, presentation, control, stats,
            item_sets, aura_catalogs, &self.loot, cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::AuraApplicationFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.represented_shapeshift_form_like_cpp,
                &self.fixtures.identity.player_class, &self.fixtures.identity.player_level,
                &self.quest_state, &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_id_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_accessories_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_seat_count_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_usable_seat_count_like_cpp,
                &mut self.fixtures.vehicles.mount_vehicle_remove_requests_like_cpp,
                &mut self.fixtures.pets.mount_pet_control_enable_requests_like_cpp,
                &mut self.fixtures.pets.mount_pet_resummon_requests_like_cpp,
                &mut self.fixtures.vehicles.mount_collision_height_update_requests_like_cpp,
            ),
        );
        wow_world_application::AppTrainerBuyAdmissionCxLikeCpp::new(
            aura, npc, &self.interaction,
            self.core.trainer_interaction_role_access_like_cpp(),
            self.catalogs.trainer_store_like_cpp().map(Arc::as_ref),
            self.core.packet_publication_access_like_cpp(),
            self.core.account_id,
        )
    }

    pub(crate) fn trainer_list_publication_context_like_cpp(&mut self) -> AppTrainerListCx<'_> {
        // Select disjoint field borrows directly: neither inert view borrows
        // the World aggregate that also owns the mutable interaction state.
        let npc = self.core.aura_npc_access_builder_like_cpp(
            self.catalogs.factions.store.as_deref(),
            self.catalogs.factions.template_store.as_deref(),
            self.catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.movement.player_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_faction_template_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_destinations_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_flight_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.taxi_mounted_like_cpp,
        );
        let role = self.core.trainer_interaction_role_access_like_cpp();
        let packets = self.core.packet_publication_access_like_cpp();
        let spell_access = self.core.owned_spell_acquisition_access_like_cpp();
        let player_conditions = wow_world_application::PlayerConditionProjectionInputsLikeCpp::new(
            self.core.aura_condition_access_builder_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_race,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_class,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_gender,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.progression.represented_primary_specialization_id_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.movement.player_position,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_zone_id_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_area_id_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_zone_area_authority_complete_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.combat.player_pvp_hostile_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.combat.player_pvp_end_timer_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.combat.player_contested_pvp_timer_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.represented_is_outdoors_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.taxi_destinations_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.taxi_mounted_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp,
            ),
            &self.social,
            self.catalogs.chr_specialization_store().map(Arc::as_ref),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.battleground,
            self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.battleground.represented_battleground_status_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_complete_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.combat.in_combat,
            #[cfg(any(test, feature = "test-fixtures"))]
            cfg!(test),
        );
        let catalogs = TrainerListCatalogsLikeCpp::new(
            self.catalogs.trainer_store_like_cpp(),
            self.catalogs
                .spell_catalogs
                .spell_acquisition_catalog()
                .map(Arc::as_ref),
            self.catalogs.skill_store().map(Arc::as_ref),
            self.catalogs.skill_line_store.as_deref(),
            self.catalogs.condition_store().map(Arc::as_ref),
            self.catalogs.player_condition_store().map(Arc::as_ref),
            wow_world_application::TrainerProjectionCatalogsLikeCpp::new(
                self.catalogs.spell_catalogs.spell_chain_store().map(Arc::as_ref),
                self.catalogs.spell_catalogs.spell_learn_skill_store_like_cpp().map(Arc::as_ref),
                self.catalogs.spell_catalogs.spell_learn_spell_store_like_cpp().map(Arc::as_ref),
                self.catalogs.spell_catalogs.spell_required_store_like_cpp().map(Arc::as_ref),
                self.catalogs.spell_catalogs.spell_custom_attribute_store_like_cpp().map(Arc::as_ref),
                self.catalogs.trait_definition_store().map(Arc::as_ref),
                self.catalogs.skill_tiers_store().map(Arc::as_ref),
                self.catalogs.mount_store().map(Arc::as_ref),
                self.catalogs.difficulty_store().map(Arc::as_ref),
                self.catalogs.map_store().map(Arc::as_ref),
                self.catalogs.disable_mgr().map(Arc::as_ref),
                self.catalogs.spell_catalogs.spell_target_restrictions_store().map(Arc::as_ref),
                self.catalogs.spell_catalogs.spell_aura_restrictions_store().map(Arc::as_ref),
                self.catalogs.spell_pet_aura_store_like_cpp(),
                self.catalogs.spell_catalogs.spell_linked_store_like_cpp(),
            ),
        );
        let presentation = self.core.player_aura_removal_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::AuraRemovalFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &mut self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                &mut self.fixtures.auras.visible_auras,
                &mut self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &mut self.fixtures.vehicles.player_mount_display_id_like_cpp,
                &mut self.fixtures.vehicles.player_mounted_like_cpp,
                &mut self.fixtures.presentation.player_unit_flags_like_cpp,
                &self.fixtures.presentation.player_object_scale_like_cpp,
            ),
        );
        let control = self.core.aura_mount_control_access_like_cpp(
            self.catalogs.creatures.display_info_store.as_deref(),
            self.catalogs.creatures.model_data_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::AuraMountControlFixtureRefsLikeCpp::new(
                &mut self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &mut self.fixtures.movement.movement_counter_like_cpp,
                &mut self.fixtures.movement.player_collision_height_like_cpp,
                &self.fixtures.movement.player_position,
                &mut self.fixtures.movement.player_movement_flags_like_cpp,
                &self.fixtures.movement.player_movement_time_like_cpp,
                &mut self.fixtures.movement.represented_can_swim_to_fly_transition_like_cpp,
                &self.fixtures.identity.player_scale_duration_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_gender,
                wow_world_core::session::AuraDismountPetFixtureRefsLikeCpp::new(
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                    &mut self.fixtures.pets.represented_pet_react_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_command_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_stable_like_cpp,
                    &mut self.fixtures.pets.represented_character_pet_rows_empty_authority_complete_like_cpp,
                    &mut self.fixtures.pets.represented_temporary_unsummoned_pet_number_like_cpp,
                    &mut self.fixtures.pets.represented_old_pet_spell_like_cpp,
                    &mut self.fixtures.pets.temporary_mount_pet_react_state_like_cpp,
                ),
                &mut self.fixtures.movement.movement_speed_rates_like_cpp,
                &mut self.fixtures.movement.forced_speed_changes_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
                &mut self.fixtures.pets.represented_pet_movement_speed_rates_like_cpp,
                &mut self.fixtures.pets.represented_pet_speed_propagations_like_cpp,
                &self.fixtures.combat.in_combat,
                &mut self.fixtures.movement.last_fall_time_like_cpp,
                &mut self.fixtures.movement.last_fall_z_like_cpp,
            ),
        );
        let stats = self.core.aura_stats_access_builder_like_cpp(
            &self.catalogs, &self.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut self.fixtures.combat.player_health_like_cpp,
                &mut self.fixtures.combat.player_max_health_like_cpp,
                &mut self.fixtures.combat.player_alive_like_cpp,
                &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
            ),
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_level,
        );
        let item_sets = self.core.owned_item_set_access_like_cpp(
            self.catalogs.items.set_store.as_deref(),
            self.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs.heirloom_store.as_deref(), self.catalogs.items.stats_store.as_deref(),
            self.catalogs.curve_store.as_deref(), self.catalogs.curve_point_store.as_deref(),
            self.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.represented_primary_specialization_id_like_cpp,
        );
        let aura_catalogs = wow_world_application::AuraApplicationCatalogsLikeCpp::new(
            self.catalogs.mount_capability_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_ref(),
            self.catalogs.chr.classes_store.as_deref(), self.catalogs.difficulty_store().map(AsRef::as_ref),
            self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(),
            self.catalogs.items.effect_store.as_ref(),
            self.catalogs.spell_catalogs.spell_shapeshift_form_store.as_deref(),
            self.catalogs.map_store().map(AsRef::as_ref),
            wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
                self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(),
                self.catalogs.scaling_stat_distribution_store.as_ref(),
                self.catalogs.scaling_stat_values_store.as_ref(),
                self.catalogs.shield_block_regular_game_table.as_ref(),
                self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
            ),
        );
        let aura = wow_world_application::PlayerAuraApplicationCxLikeCpp::new(
            &mut self.spell_state, &mut self.inventory, presentation, control, stats,
            item_sets, aura_catalogs, &self.loot, cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::AuraApplicationFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.represented_shapeshift_form_like_cpp,
                &self.fixtures.identity.player_class, &self.fixtures.identity.player_level,
                &self.quest_state, &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_id_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_accessories_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_seat_count_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_usable_seat_count_like_cpp,
                &mut self.fixtures.vehicles.mount_vehicle_remove_requests_like_cpp,
                &mut self.fixtures.pets.mount_pet_control_enable_requests_like_cpp,
                &mut self.fixtures.pets.mount_pet_resummon_requests_like_cpp,
                &mut self.fixtures.vehicles.mount_collision_height_update_requests_like_cpp,
            ),
        );
        let account_id = self.core.account_id;
        let session_locale_name = self.core.session_locale_name_like_cpp();
        AppTrainerListCx::new(
            &mut self.interaction,
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
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_complete_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_occupied_slots_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_non_durable_tombstones_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_loaded_like_cpp,
            &self.config.max_primary_trade_skills_like_cpp,
        )
    }
}
