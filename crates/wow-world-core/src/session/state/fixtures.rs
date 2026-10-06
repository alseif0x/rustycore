// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::fixtures` (#1241 F3-0): the cfg(test) groups, moved unchanged.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::{
    AuraState, BattlegroundState, CollectionsState, CombatState, MovementState, PetState,
    PlayerIdentityState, PlayerPresentationState, ProgressionState, TaxiVehicleState,
    TeleportState,
};
use crate::session::movement_protocol::UnitMoveTypeLikeCpp;
use crate::session::persistence_capabilities::empty_character_power_snapshot_like_cpp;
use crate::session::pets::test_fixtures::BattlePetTestFixtureLikeCpp;
use crate::session::{
    MAX_SPECIALIZATIONS_LIKE_CPP, PlayerSkillTestFixtureLikeCpp, RestMgrTestFixtureLikeCpp,
};
use wow_constants::{MovementFlag, UnitFlags, UnitStandStateType};
use wow_core::ObjectGuid;
use wow_entities::PetStable;

/// Test-only fixture groups (#1241 F3-0): the 11 cfg(test) domain groups, nested unchanged so an F3
/// context borrows one member instead of eleven.
pub struct SessionFixtures {
    /// Player identity fixtures: race, class, level, gender, name, create mode, faction template,
    /// scale and zone/area state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub identity: PlayerIdentityState,
    /// Account collections: mounts, heirlooms, toys, item appearances, transmog illusions and
    /// completed achievements.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub collections: CollectionsState,
    /// Player aura fixtures: visible auras, aura authority, the spell-hit tombstone, threat-aura
    /// snapshots and the shapeshift form.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub auras: AuraState,
    /// XP, talents, glyphs and respec, skills and proficiencies, reputation and rest fixtures.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub progression: ProgressionState,
    /// Combat target and flags, vitals and powers, GM and immunity flags, PvP flags and timers,
    /// death and resurrection.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub combat: CombatState,
    /// Player movement fixtures: position, flags, jump and fall, acks, speeds and force mods, and
    /// vehicle movement sinks.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub movement: MovementState,
    /// Near, far and delayed teleport state and acks, the pending teleport, the homebind and
    /// spline-done taxi events.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub teleport: TeleportState,
    /// Taxi flight and mount/vehicle kit state: destinations, seat state, vehicle requests,
    /// transport attach and mount counters.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub vehicles: TaxiVehicleState,
    /// Represented pet state, pet stable, react and command state, pet speeds, temporary (un)summon
    /// and mount pet-control counters.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub pets: PetState,
    /// Battleground and arena membership and the represented battlemaster, battlefield and wargame
    /// request sinks.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub battleground: BattlegroundState,
    /// Player presentation fixtures: unit flags and scale, stand state and emote, action bars,
    /// cinematics, CUF profiles and barber requests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub presentation: PlayerPresentationState,
}

impl Default for SessionFixtures {
    fn default() -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            identity: PlayerIdentityState {
                #[cfg(any(test, feature = "test-fixtures"))]
                player_race: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_class: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_level: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_gender: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_create_mode_like_cpp: wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_name: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_scale_duration_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_faction_template_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_zone_id_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_area_id_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_zone_area_authority_complete_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_is_outdoors_like_cpp: None,
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            collections: CollectionsState {
                #[cfg(any(test, feature = "test-fixtures"))]
                account_mounts_like_cpp: HashMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_account_heirlooms_like_cpp: BTreeMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_account_toys_like_cpp: BTreeMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_item_appearances_like_cpp: HashSet::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_item_appearance_blocks_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_temporary_item_appearances_like_cpp: HashMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_favorite_item_appearances_like_cpp: HashMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_transmog_illusions_like_cpp: HashSet::new(),

                #[cfg(any(test, feature = "test-fixtures"))]
                represented_completed_achievements_like_cpp: HashSet::new(),
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            auras: AuraState {
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_shapeshift_form_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_spell_hit_aura_authority_tombstoned_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                visible_auras: HashMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                player_aura_authority_complete_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                canonical_threat_aura_snapshots_like_cpp: HashMap::new(),
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            progression: ProgressionState {
                #[cfg(any(test, feature = "test-fixtures"))]
                championing_faction_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_enchanting_skill: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_skill_test_fixture_like_cpp: PlayerSkillTestFixtureLikeCpp::default(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_gray_level_script_overrides_like_cpp: HashMap::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                rest_mgr_test_fixture_like_cpp: RestMgrTestFixtureLikeCpp::default(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talent_reset_cost_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talent_reset_time_secs_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_character_points_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_xp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_next_level_xp: 400,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_primary_specialization_id_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_weapon_proficiency_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_armor_proficiency_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_confirm_respec_wipe_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talent_reset_script_hooks_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talent_respec_visual_spell_casts_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talent_respec_criteria_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_active_talent_group_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_bonus_talent_groups_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talents_like_cpp: std::array::from_fn(|_| BTreeMap::new()),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_talents_loaded_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_glyphs_like_cpp: [[0;
                    wow_packet::packets::misc::MAX_GLYPH_SLOT_INDEX_LIKE_CPP];
                    MAX_SPECIALIZATIONS_LIKE_CPP],
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_glyphs_loaded_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                #[cfg(any(test, feature = "test-fixtures"))]
                reputation_state_like_cpp: wow_entities::PlayerReputationStateLikeCpp::default(),
                #[cfg(any(test, feature = "test-fixtures"))]
                watched_faction_index_like_cpp: -1,
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            combat: CombatState {
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_player_powers_like_cpp: empty_character_power_snapshot_like_cpp(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_player_max_powers_like_cpp: empty_character_power_snapshot_like_cpp(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_player_base_mana_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                selection_guid: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                combat_target: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                in_combat: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_alive_like_cpp: true,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_game_master_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_cheat_god_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_normal_damage_immune_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_environmental_damage_immune_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_health_like_cpp: 100,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_max_health_like_cpp: 100,
                #[cfg(any(test, feature = "test-fixtures"))]
                area_spirit_healer_guid_like_cpp: ObjectGuid::EMPTY,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_pvp_hostile_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_pvp_enabled_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_in_pvp_flag_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_pvp_end_timer_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_contested_pvp_timer_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_resurrection_request_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_delayed_resurrection_after_teleport_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_death_timer_active_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_repop_at_graveyard_count: 0,
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            movement: MovementState {
                #[cfg(any(test, feature = "test-fixtures"))]
                player_position: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_movement_flags_like_cpp: MovementFlag::NONE,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_can_swim_to_fly_transition_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_mover_fixed_position_vehicle_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_movement_time_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_movement_jump_like_cpp: wow_packet::packets::movement::JumpInfo::default(),
                #[cfg(any(test, feature = "test-fixtures"))]
                last_fall_time_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                last_fall_z_like_cpp: 0.0,
                #[cfg(any(test, feature = "test-fixtures"))]
                fall_damage_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                player_out_of_bounds_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                under_map_damage_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                movement_jump_proc_requests_like_cpp: 0,

                #[cfg(any(test, feature = "test-fixtures"))]
                player_moved_unit_guid_like_cpp: ObjectGuid::EMPTY,

                #[cfg(any(test, feature = "test-fixtures"))]
                movement_ack_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_vehicle_dismiss_movements_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_vehicle_base_movements_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                movement_counter_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_collision_height_like_cpp: 1.0,
                #[cfg(any(test, feature = "test-fixtures"))]
                delayed_operations_processed_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                forced_speed_changes_like_cpp: [0; UnitMoveTypeLikeCpp::COUNT],
                #[cfg(any(test, feature = "test-fixtures"))]
                movement_speed_rates_like_cpp: [1.0; UnitMoveTypeLikeCpp::COUNT],
                #[cfg(any(test, feature = "test-fixtures"))]
                movement_force_mod_magnitude_changes_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                movement_force_mod_magnitude_like_cpp: 1.0,
                #[cfg(any(test, feature = "test-fixtures"))]
                movement_speed_ack_events_like_cpp: Vec::new(),
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            teleport: TeleportState {
                #[cfg(any(test, feature = "test-fixtures"))]
                move_spline_done_taxi_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_can_delay_teleport_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_has_delayed_teleport_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                near_teleport_pending_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_far_teleport_pending_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                near_teleport_destination_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_delayed_teleport_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                near_teleport_destination_zone_area_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_homebind_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                move_teleport_ack_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                pending_teleport: None,
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            vehicles: TaxiVehicleState {
                #[cfg(any(test, feature = "test-fixtures"))]
                taxi_destinations_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_activate_taxi_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                taxi_flight_state_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                taxi_unit_flags_like_cpp: UnitFlags::empty(),
                #[cfg(any(test, feature = "test-fixtures"))]
                taxi_mounted_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mount_display_id_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mount_vehicle_id_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mount_vehicle_kit_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mount_vehicle_accessories_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mount_vehicle_seat_count_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mount_vehicle_usable_seat_count_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_vehicle_seat_flags_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_vehicle_seat_id_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_vehicle_seat_change_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_vehicle_seat_spell_click_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_vehicle_enter_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_vehicle_create_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_vehicle_remove_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_cancel_expected_vehicle_aura_packets_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_collision_height_update_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_mounted_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_on_transport_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_transport_login_state_like_cpp: None,
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            pets: PetState {
                #[cfg(any(test, feature = "test-fixtures"))]
                temporary_pet_unsummon_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_guid_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_temporary_unsummoned_pet_number_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_old_pet_spell_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_stable_like_cpp: PetStable::default(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_character_pet_rows_empty_authority_complete_like_cpp: false,

                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_created_by_spell_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_react_state_like_cpp:
                    wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_command_state_like_cpp:
                    wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
                #[cfg(any(test, feature = "test-fixtures"))]
                temporary_mount_pet_react_state_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_pet_control_disable_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_pet_control_enable_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                mount_pet_resummon_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                temporary_pet_resummon_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_movement_speed_rates_like_cpp: [1.0; UnitMoveTypeLikeCpp::COUNT],
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_pet_speed_propagations_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                battle_pet_test_fixture_like_cpp: BattlePetTestFixtureLikeCpp::default(),
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            battleground: BattlegroundState {
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_arena_team_id_invited_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_wargame_invite_acceptances_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                player_battleground_type_id_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_battleground_map_id_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battleground_status_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battleground_leave_requests_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battlemaster_hellos_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battlefield_lists_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battlemaster_joins_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battlemaster_join_arenas_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battlemaster_join_skirmishes_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battleground_queue_slots_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_battlefield_ports_like_cpp: Vec::new(),
            },
            #[cfg(any(test, feature = "test-fixtures"))]
            presentation: PlayerPresentationState {
                #[cfg(any(test, feature = "test-fixtures"))]
                cuf_profiles_like_cpp: vec![
                    None;
                    wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP
                ],
                #[cfg(any(test, feature = "test-fixtures"))]
                cuf_profiles_loaded_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_stand_state_like_cpp: UnitStandStateType::Stand,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_live_applications_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                player_emote_state_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                active_player_local_flags_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                active_player_transport_server_time_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                active_player_multi_action_bars_like_cpp: 0,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_action_buttons_like_cpp: [0;
                    wow_packet::packets::misc::MAX_ACTION_BUTTONS],
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_action_buttons_loaded_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_alter_appearance_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_confirm_barbers_choice_requests_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                player_object_scale_like_cpp: 1.0,
                #[cfg(any(test, feature = "test-fixtures"))]
                player_unit_flags_like_cpp: UnitFlags::PLAYER_CONTROLLED,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_cinematic_state_like_cpp:
                    wow_entities::PlayerCinematicStateLikeCpp::default(),
                #[cfg(any(test, feature = "test-fixtures"))]
                #[cfg(any(test, feature = "test-fixtures"))]
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_cinematic_next_camera_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_cinematic_end_events_like_cpp: Vec::new(),
                #[cfg(any(test, feature = "test-fixtures"))]
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_movie_complete_events_like_cpp: Vec::new(),
            },
        }
    }
}
