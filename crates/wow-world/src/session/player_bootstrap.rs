// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player bootstrap: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::power_type_from_u8_like_cpp;
use super::{INVENTORY_DEFAULT_SIZE, PLAYER_FLAGS_IN_PVP_LIKE_CPP, PhaseShift, Player};
use super::{PlayerPetLifecycleStateLikeCpp, PlayerResurrectionStateLikeCpp};
use super::{
    PlayerTeleportStateLikeCpp, Position, RepresentedPlayerSpellRuntimeLikeCpp, UnitFlags,
};
use super::{UnitPvpFlags, WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP, WeaponAttackType};
use super::{WorldSession, gender_from_u8, player_cuf_profile_from_packet_like_cpp};
#[cfg(test)]
use super::{
    canonical_player_spell_runtime_like_cpp, primary_power_type_for_player_class_like_cpp,
};

impl WorldSession {
    /// Build the initial canonical Player value before a generation-checked
    /// owner exists. This is construction input only: once a handle has been
    /// installed, callers must query or mutate that exact owner in place.
    pub(in crate::session) fn build_initial_player_for_owner_like_cpp(
        &self,
        key: wow_map::MapKey,
        bootstrap_position: Option<Position>,
    ) -> Option<Player> {
        #[cfg(not(test))]
        if self.player_handle_like_cpp.is_some() {
            return None;
        }

        let guid = self.player_guid()?;
        let position = bootstrap_position.or_else(|| self.player_position_like_cpp())?;
        let name = self.player_name_like_cpp()?;
        let mut player = Player::new(Some(u64::from(self.account_id)), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name(name);
        player
            .unit_mut()
            .world_mut()
            .set_map(key.map_id, key.instance_id)
            .ok()?;
        player.unit_mut().world_mut().relocate(position);
        // Before the first canonical owner exists production has no phase
        // authority to read. The pre-load bootstrap has always been the C++
        // default empty PhaseShift; tests may provide an explicit fixture.
        #[cfg(not(test))]
        let bootstrap_phase_shift = PhaseShift::default();
        #[cfg(test)]
        let bootstrap_phase_shift = self
            .visibility_test_fixture_like_cpp
            .represented_player_phase_shift
            .clone();
        *player.unit_mut().world_mut().phase_shift_mut() = bootstrap_phase_shift;
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.set_race_class_gender(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            gender_from_u8(self.player_gender_like_cpp()),
        );
        if let Some(faction_template) = self.player_faction_template_id_like_cpp() {
            player.unit_mut().set_faction(faction_template);
        }
        player.unit_mut().set_level(self.player_level_like_cpp());
        // Preserve the pre-load Rust bootstrap shape. The Character row later
        // hydrates these values directly into the generation-checked Player;
        // Session no longer stores a second production vital-state authority.
        #[cfg(not(test))]
        {
            player.unit_mut().set_max_health(100);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(100);
        }
        #[cfg(test)]
        {
            player
                .unit_mut()
                .set_max_health(u64::from(self.player_max_health_like_cpp.max(1)));
            if !self.player_alive_like_cpp || self.player_health_like_cpp == 0 {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
            } else {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
            }
            player
                .unit_mut()
                .set_health(u64::from(self.player_health_like_cpp));
        }
        #[cfg(test)]
        self.apply_represented_player_powers_to_canonical_like_cpp(&mut player);
        #[cfg(not(test))]
        {
            // This value is the pre-Character-row bootstrap only. Production
            // immediately hydrates the same owned Player through the setters
            // below; it is never used as a fallback for an unresolved handle.
            player.set_xp(0);
            player.set_next_level_xp(400);
            player.set_character_points_like_cpp(0);
        }
        #[cfg(test)]
        {
            player.set_xp(self.player_xp_like_cpp() as i32);
            player.set_next_level_xp(self.player_next_level_xp_like_cpp() as i32);
            player.set_character_points_like_cpp(self.player_character_points_like_cpp());
        }
        #[cfg(not(test))]
        player.set_scaling_player_level_delta_like_cpp(
            if self.player_level_like_cpp() < WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP {
                -1
            } else {
                0
            },
        );
        #[cfg(test)]
        player.set_scaling_player_level_delta_like_cpp(self.player_scaling_level_delta_like_cpp());
        #[cfg(not(test))]
        player.set_money(0);
        #[cfg(test)]
        player.set_money(self.player_gold_like_cpp());
        #[cfg(not(test))]
        {
            // Pre-character-row bootstrap only. Login hydrates both values
            // into this same owned Player before gameplay admission.
            player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);
            player.set_bank_bag_slot_count(0);
        }
        #[cfg(test)]
        {
            player.set_inventory_slot_count(
                self.player_item_test_fixture_like_cpp
                    .player_inventory_slot_count_like_cpp,
            );
            player.set_bank_bag_slot_count(
                self.player_item_test_fixture_like_cpp
                    .player_bank_bag_slot_count_like_cpp,
            );
        }
        for (category, party_type) in self
            .party_member_party_type_like_cpp()
            .into_iter()
            .enumerate()
        {
            let _ = player.set_party_type_like_cpp(category as u8, party_type);
        }
        #[cfg(test)]
        for (index, value) in self
            .represented_bank_bag_slot_flags_like_cpp
            .iter()
            .copied()
            .enumerate()
        {
            player.set_bank_bag_slot_flag_value_like_cpp(index, value);
        }
        #[cfg(not(test))]
        player.set_watched_faction_index_like_cpp(-1);
        #[cfg(test)]
        player.set_watched_faction_index_like_cpp(self.watched_faction_index_like_cpp);
        #[cfg(test)]
        for quest_bit in &self
            .quest_test_fixture_like_cpp
            .represented_quest_completed_bits_like_cpp
        {
            player.set_quest_completed_bit_like_cpp(*quest_bit, true);
        }
        #[cfg(test)]
        player.set_explored_zones_blocks_like_cpp(&self.represented_explored_zones_like_cpp);
        #[cfg(test)]
        {
            player.gameplay_state_mut().world_local =
                wow_entities::PlayerWorldLocalState::from_represented_parts_like_cpp(
                    self.player_zone_id_like_cpp,
                    self.player_area_id_like_cpp,
                    self.player_zone_area_authority_complete_like_cpp,
                    self.player_pvp_hostile_like_cpp,
                    self.player_pvp_end_timer_like_cpp,
                    self.player_contested_pvp_timer_like_cpp,
                    self.represented_is_outdoors_like_cpp,
                );
            player.gameplay_state_mut().vehicle_seat_flags =
                self.player_vehicle_seat_flags_like_cpp;
            player.gameplay_state_mut().vehicle_seat_id = self.player_vehicle_seat_id_like_cpp;
            player.gameplay_state_mut().active_local_flags =
                self.active_player_local_flags_like_cpp;
            player.gameplay_state_mut().active_transport_server_time =
                self.active_player_transport_server_time_like_cpp;
            player.gameplay_state_mut().multi_action_bars =
                self.active_player_multi_action_bars_like_cpp;
            player
                .unit_mut()
                .subsystems_mut()
                .control
                .set_moved_unit(Some(if self.player_moved_unit_guid_like_cpp.is_empty() {
                    guid
                } else {
                    self.player_moved_unit_guid_like_cpp
                }));
            player
                .unit_mut()
                .world_mut()
                .set_zone_and_area(self.player_zone_id_like_cpp, self.player_area_id_like_cpp);
            if self.player_pvp_enabled_like_cpp {
                player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
            }
            if self.player_in_pvp_flag_like_cpp {
                player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
            }
        }
        #[cfg(not(test))]
        player
            .unit_mut()
            .subsystems_mut()
            .control
            .set_moved_unit(Some(guid));
        #[cfg(test)]
        player.set_game_master_like_cpp(self.player_game_master_like_cpp);
        #[cfg(test)]
        {
            player.set_cheat_god_like_cpp(self.player_cheat_god_like_cpp);
            player.set_normal_damage_immune_like_cpp(self.player_normal_damage_immune_like_cpp);
            player.set_environmental_damage_immune_like_cpp(
                self.player_environmental_damage_immune_like_cpp,
            );
            *player.resurrection_state_mut_like_cpp() = PlayerResurrectionStateLikeCpp {
                request: self.represented_resurrection_request_like_cpp,
                delayed_after_teleport: self
                    .represented_delayed_resurrection_after_teleport_like_cpp,
                self_res_spells: self.represented_self_res_spells_like_cpp.clone(),
                death_timer_active: self.represented_death_timer_active_like_cpp,
                area_spirit_healer_guid: self.area_spirit_healer_guid_like_cpp,
            };
            *player.teleport_state_mut_like_cpp() = PlayerTeleportStateLikeCpp {
                recovery: Default::default(),
                far_destination: self.pending_teleport,
                post_add: None,
                can_delay: self.represented_can_delay_teleport_like_cpp,
                has_delayed: self.represented_has_delayed_teleport_like_cpp,
                near_pending: self.near_teleport_pending_like_cpp,
                far_pending: self.represented_far_teleport_pending_like_cpp,
                near_destination: self.near_teleport_destination_like_cpp,
                delayed: self.represented_delayed_teleport_like_cpp,
                near_destination_zone_area: self.near_teleport_destination_zone_area_like_cpp,
            };
            *player.pet_lifecycle_state_mut_like_cpp() = PlayerPetLifecycleStateLikeCpp {
                stable: self.represented_pet_stable_like_cpp.clone(),
                character_rows_empty_authority_complete: self
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                temporary_unsummoned_pet_number: self
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                old_pet_spell: self.represented_old_pet_spell_like_cpp,
                temporary_mount_react_state: self.temporary_mount_pet_react_state_like_cpp,
            };
        }
        crate::canonical_player_sync::hydrate_player_presentation_like_cpp(self, &mut player)?;
        #[cfg(test)]
        {
            player.set_create_mode_like_cpp(self.player_create_mode_like_cpp);
            player.set_shapeshift_form_id_like_cpp(self.represented_shapeshift_form_like_cpp);
            player.set_loot_specialization_id_like_cpp(self.loot_specialization_id);
            player.set_primary_specialization(self.represented_primary_specialization_id_like_cpp);
            player.replace_spell_runtime_like_cpp(canonical_player_spell_runtime_like_cpp(
                RepresentedPlayerSpellRuntimeLikeCpp {
                    known_spells: self.player_spell_test_fixture_like_cpp.known_spells.clone(),
                    rows: self
                        .player_spell_test_fixture_like_cpp
                        .represented_player_spell_rows_like_cpp
                        .clone(),
                    rows_loaded: self
                        .player_spell_test_fixture_like_cpp
                        .represented_player_spell_rows_loaded_like_cpp,
                    rows_complete: self
                        .player_spell_test_fixture_like_cpp
                        .represented_player_spell_rows_complete_like_cpp,
                    fallback_rows: self
                        .player_spell_test_fixture_like_cpp
                        .represented_fallback_player_spell_rows_like_cpp
                        .clone(),
                    dependent_known_spells: self
                        .player_spell_test_fixture_like_cpp
                        .represented_dependent_known_spells_like_cpp
                        .clone(),
                    removed_known_spells: self
                        .player_spell_test_fixture_like_cpp
                        .represented_removed_known_spells_like_cpp
                        .clone(),
                    favorite_known_spells: self
                        .player_spell_test_fixture_like_cpp
                        .represented_favorite_known_spells_like_cpp
                        .clone(),
                    trait_definition_ids: self
                        .player_spell_test_fixture_like_cpp
                        .represented_spell_trait_definition_ids_like_cpp
                        .clone(),
                    trait_definition_ids_complete: self
                        .player_spell_test_fixture_like_cpp
                        .represented_spell_trait_definition_ids_complete_like_cpp,
                    trait_config_rows: self
                        .player_spell_test_fixture_like_cpp
                        .represented_trait_config_rows_like_cpp
                        .clone(),
                    trait_config_rows_complete: self
                        .player_spell_test_fixture_like_cpp
                        .represented_trait_config_rows_complete_like_cpp,
                    trait_entry_rows_complete: self
                        .player_spell_test_fixture_like_cpp
                        .represented_trait_entry_rows_complete_like_cpp,
                    trait_entry_rows_empty: self
                        .player_spell_test_fixture_like_cpp
                        .represented_trait_entry_rows_empty_like_cpp,
                    override_spells: self.represented_override_spells_like_cpp.clone(),
                    override_spells_complete: self.represented_override_spells_complete_like_cpp,
                },
            ));
            player.gameplay_state_mut().cuf_profiles = self
                .cuf_profiles_like_cpp
                .iter()
                .map(|profile| profile.clone().map(player_cuf_profile_from_packet_like_cpp))
                .collect();
            player.gameplay_state_mut().cuf_profiles_loaded = self.cuf_profiles_loaded_like_cpp;
            player.gameplay_state_mut().equipment_sets =
                self.represented_equipment_sets_like_cpp.clone();
            player.gameplay_state_mut().void_storage_items =
                self.represented_void_storage_items_like_cpp.to_vec();
            player.gameplay_state_mut().void_storage_loaded =
                self.represented_void_storage_loaded_like_cpp;
            player.gameplay_state_mut().collections =
                self.represented_player_collection_state_like_cpp();
        }
        player
            .unit_mut()
            .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
        #[cfg(not(test))]
        player
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::PLAYER_CONTROLLED);
        #[cfg(test)]
        player
            .unit_mut()
            .set_unit_flags_like_cpp(self.player_unit_flags_like_cpp);
        if let Some(selection) = self.selection_guid_like_cpp() {
            player.set_selection(selection);
        }
        self.apply_represented_player_unit_shape_to_canonical_like_cpp(&mut player);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .set_spell_hit_aura_authority_inert_like_cpp(
                self.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
            );
        Some(player)
    }

    #[cfg(test)]
    pub(in crate::session) fn apply_represented_player_powers_to_canonical_like_cpp(
        &self,
        player: &mut Player,
    ) {
        let powers = self
            .represented_player_powers_like_cpp
            .map(|value| value.unwrap_or(0));
        let max_powers = self
            .represented_player_max_powers_like_cpp
            .map(|value| value.unwrap_or(0));
        player
            .unit_mut()
            .replace_create_power_arrays_like_cpp(powers, max_powers);
        let Some(current) = self.represented_player_powers_like_cpp[0] else {
            return;
        };
        let primary_power_type =
            primary_power_type_for_player_class_like_cpp(self.player_class_like_cpp());
        for raw_power in 0..=25 {
            player.set_power_index(power_type_from_u8_like_cpp(raw_power), None);
        }
        player.set_power_index(primary_power_type, Some(0));
        player.unit_mut().set_display_power(primary_power_type);
        player
            .unit_mut()
            .set_create_mana_like_cpp(self.represented_player_base_mana_like_cpp.max(0));
        if let Some(max) = self.represented_player_max_powers_like_cpp[0] {
            player.unit_mut().set_max_power(primary_power_type, max);
            player.unit_mut().set_power(primary_power_type, current);
        }
    }

    pub(in crate::session) fn apply_represented_player_unit_shape_to_canonical_like_cpp(
        &self,
        player: &mut Player,
    ) {
        let display_id = crate::handlers::character::default_display_id(
            self.player_race_like_cpp(),
            self.player_gender_like_cpp(),
        );
        #[cfg(not(test))]
        let mount_display_id = 0;
        #[cfg(test)]
        let mount_display_id = u32::try_from(self.player_mount_display_id_like_cpp).unwrap_or(0);
        let unit = player.unit_mut();
        unit.set_display_id(display_id, true);
        unit.set_mount_display_id(mount_display_id);
        #[cfg(not(test))]
        unit.set_collision_height_like_cpp(1.0);
        #[cfg(test)]
        unit.set_collision_height_like_cpp(self.player_collision_height_like_cpp);
        #[cfg(not(test))]
        unit.world_mut().object_mut().set_scale(1.0);
        #[cfg(test)]
        unit.world_mut()
            .object_mut()
            .set_scale(self.player_object_scale_like_cpp);
    }

    #[inline(never)]
    pub(in crate::session) fn initial_player_box_like_cpp(
        &self,
        key: wow_map::MapKey,
    ) -> Option<Box<Player>> {
        Some(Box::new(
            self.build_initial_player_for_owner_like_cpp(key, None)?,
        ))
    }
}
