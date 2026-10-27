//! Session-independent world-entity state and represented creature publication.

pub(crate) const PLAYER_LOCAL_FLAG_RELEASE_TIMER: u32 = 0x0000_0008;

mod aggro;
mod catalogs;
mod combat_handlers;
mod combat_session;
mod contracts;
mod corpse_handlers;
mod corpse_login;
mod corpse_registrations;
mod creature;
pub mod creature_aggro_contracts;
mod creature_aura_tracking;
mod creature_interaction;
mod creature_kill;
mod creature_kill_session;
pub mod creature_movement_adapter;
mod creature_publication;
mod creature_publication_session;
mod creature_query;
mod creature_registry;
pub mod creature_spell_admission;
pub mod creature_spell_metadata;
pub mod creature_spell_planning;
pub mod creature_spell_publication;
mod creature_visibility;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod gameobject;
mod gameobject_access;
mod gameobject_contracts;
mod gameobject_handlers;
mod gameobject_interaction;
mod gameobject_overrides;
mod gameobject_publication_session;
mod gameobject_query;
mod gameobject_state;
mod gameobject_update_session;
mod gameobject_use;
mod gameobject_use_dispatch;
mod gameobject_use_kinds;
mod gameobject_use_preamble;
mod gameobject_use_pvp;
mod gameobject_use_scripted;
mod gameobject_use_support;
mod gameobject_use_types;
mod module_hooks;
mod movement_acks;
mod movement_handlers;
mod movement_host;
mod movement_opcodes;
mod pending_runtime;
mod player_attack;
mod player_death;
mod player_release;
mod player_resurrection;
mod recovery_fences;
mod player_vitals;
mod resurrection_request;
mod spawn;
mod spell_click;
pub mod spell_click_values;
mod spirit_healer;
mod state;
mod teleport_session;
mod travel_handlers;
mod vehicle_handlers;
mod vehicle_session;
mod visibility_session;
mod world_entities_host;

pub use aggro::{
    creature_threat_value_on_map_like_cpp, mirror_creature_threat_from_attacker_on_map_like_cpp,
};
pub use catalogs::CreatureSpawnCatalogsLikeCpp;
pub use combat_handlers::{
    CombatHandlerCxLikeCpp, CombatHandlerHostLikeCpp, register_combat_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use combat_session::send_player_power_update_like_cpp;
pub use combat_session::{
    CombatSessionHostLikeCpp, apply_represented_ffa_pvp_login_state_like_cpp, combat_stop_like_cpp,
    handle_apply_creature_melee_damage_like_cpp_command_like_cpp,
    handle_creature_attack_start_like_cpp_command_like_cpp,
    handle_creature_attack_stop_like_cpp_command_like_cpp,
    handle_reconcile_pvp_combat_expiry_like_cpp, reset_contested_pvp_like_cpp,
    set_represented_ffa_pvp_flag_like_cpp, tick_player_regeneration_like_cpp,
    update_pvp_flag_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedCreatureKillEventLikeCpp;
pub use contracts::{
    CreatureCreateStatsLikeCpp, MaterializedCreatureSpawnLikeCpp, PendingCreatureKillRewardLikeCpp,
    PendingCreatureSpawn, RepresentedCreatureAuraLikeCpp,
    RepresentedSpellClickCreatureSnapshotLikeCpp,
};
pub use corpse_handlers::{
    CorpseHostLikeCpp, ResurrectionSaveOutcome, handle_reclaim_corpse, handle_repop_request,
    handle_request_cemetery_list_with_catalog_like_cpp, handle_resurrect_response,
};
pub use corpse_login::restore_player_corpse_after_login;
pub use module_hooks::CreatureKillHookEventLikeCpp;
pub use spirit_healer::send_spirit_resurrect;
// The exact `ApplicationCorpse` owner facade the handler-contract checker
// requires (#1263 F5 deferred families).
pub use corpse_registrations::{CorpseHandlerHostLikeCpp, register_corpse_handlers_like_cpp};
pub use creature::creature_message_to_set_target_allows_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use creature_kill_session::record_represented_creature_kill_hooks_like_cpp;
pub use creature_kill_session::{
    complete_represented_creature_death_state_after_kill_hooks_like_cpp,
    process_pending_creature_kills_with_generator_like_cpp,
    represented_creature_kill_reputation_rate_like_cpp,
    reward_reputation_from_creature_kill_like_cpp,
};
pub use creature_publication::represented_creature_aura_info_like_cpp;
pub use creature_publication_session::{
    apply_creature_aura_with_provenance_like_cpp,
    broadcast_creature_packet_to_visible_set_like_cpp,
    represented_can_receive_creature_message_to_set_by_guid_like_cpp,
    represented_can_receive_creature_message_to_set_like_cpp,
    tick_represented_creature_auras_like_cpp,
};
pub use creature_registry::{
    CanonicalCreatureInsertOutcomeLikeCpp,
    insert_canonical_creature_map_object_on_locked_map_like_cpp,
    insert_canonical_creature_map_object_on_map_like_cpp, sheath_state_from_u8_like_cpp,
    unit_stand_state_from_u8_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use gameobject_contracts::RepresentedGameObjectCriteriaEvent;
pub use gameobject_contracts::{
    BattlegroundFlagDropClickTarget, RepresentedBattlegroundObjectUseRejection,
    RepresentedCapturePointStateLikeCpp, RepresentedGameObjectSpellCaster,
    RepresentedGameObjectUseEffect, RepresentedGameObjectUseState, RepresentedNewFlagStateRequest,
};
pub use gameobject_handlers::{
    GameObjectHandlerCxLikeCpp, GameObjectHandlerHostLikeCpp, register_gameobject_handlers_like_cpp,
};
pub use gameobject_interaction::{
    represented_gameobject_display_box_contains_like_cpp,
    represented_gameobject_interaction_distance_like_cpp,
};
pub use gameobject_publication_session::{
    send_represented_capture_point_removed_from_last_update_like_cpp,
    send_represented_gameobject_visual_despawn_from_last_update_like_cpp,
};
pub use gameobject_update_session::tick_represented_gameobject_update_like_cpp;
pub use gameobject_use_dispatch::{
    handle_game_obj_use_with_catalogs_like_cpp,
    represented_gameobject_gossip_can_interact_with_like_cpp,
    represented_gameobject_icon_allows_interaction_like_cpp,
};
pub use gameobject_use_kinds::{
    use_represented_gameobject_barber_chair_like_cpp, use_represented_gameobject_camera_like_cpp,
    use_represented_gameobject_chair_like_cpp, use_represented_gameobject_spellcaster_like_cpp,
    use_represented_gameobject_ui_link_like_cpp,
};
pub use gameobject_use_preamble::{
    apply_represented_gameobject_goober_just_deactivated_like_cpp,
    apply_represented_gameobject_player_use_preamble_like_cpp,
    apply_represented_gameobject_post_use_spell_like_cpp,
    represented_gameobject_can_interact_with_like_cpp,
    represented_gameobject_use_allowed_by_mover_like_cpp,
};
pub use gameobject_use_pvp::{
    use_represented_gameobject_capture_point_like_cpp,
    use_represented_gameobject_flagdrop_like_cpp, use_represented_gameobject_flagstand_like_cpp,
    use_represented_gameobject_new_flag_drop_like_cpp,
    use_represented_gameobject_new_flag_like_cpp,
};
pub use gameobject_use_scripted::{
    SKILL_FISHING_LIKE_CPP, use_represented_gameobject_fishing_node_like_cpp,
    use_represented_gameobject_goober_preamble_with_generator_like_cpp,
    use_represented_gameobject_goober_state_like_cpp,
    use_represented_gameobject_meeting_stone_like_cpp, use_represented_gameobject_ritual_like_cpp,
};
pub use gameobject_use_support::{
    apply_represented_new_flag_state_command_like_cpp,
    record_represented_capture_point_update_like_cpp,
    record_represented_gameobject_owner_guid_like_cpp,
    remove_represented_stealth_or_invisibility_auras_by_type_like_cpp,
    represented_fishing_base_skill_level_like_cpp,
    represented_gameobject_is_friendly_to_player_like_cpp,
    represented_player_battleground_type_id_or_reject_like_cpp,
    represented_player_can_use_battleground_object_like_cpp,
    represented_player_reject_battleground_object_vehicle_like_cpp,
    send_represented_gameobject_delete_packets_like_cpp,
    send_represented_gameobject_despawn_to_visible_set_like_cpp,
    use_represented_gameobject_item_forge_like_cpp,
};
pub use movement_acks::{
    apply_knock_back_ack_like_cpp, apply_move_set_vehicle_rec_id_ack_like_cpp,
    handle_fall_like_cpp, handle_force_speed_change_ack_like_cpp,
    handle_move_spline_done_taxi_like_cpp, handle_movement_force_mod_magnitude_ack_like_cpp,
    handle_under_map_like_cpp, record_apply_movement_force_ack_like_cpp,
    record_remove_movement_force_ack_like_cpp, record_validated_movement_ack_like_cpp,
    sanitize_movement_info_flags_represented_like_cpp, sanitize_movement_info_represented_like_cpp,
};
pub use movement_handlers::register_movement_tail_handlers_like_cpp;
pub use movement_handlers::{
    MovementHandlerCxLikeCpp, MovementHandlerHostLikeCpp, register_movement_handlers_like_cpp,
};
pub use movement_host::PlayerMovementHostLikeCpp;
pub use movement_opcodes::{
    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP,
    SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
    apply_movement_side_effects_like_cpp, handle_move_spline_done, handle_movement_ack_message,
    handle_movement_info_with_catalogs_like_cpp, handle_movement_with_catalogs_like_cpp,
};
pub use player_attack::{
    PLAYER_FLAGS_UBER_LIKE_CPP, PlayerAttackStartLikeCppResult,
    handle_apply_player_melee_result_like_cpp_command_like_cpp, handle_attack_swing,
    start_player_attack_like_cpp,
};
pub use player_death::{
    CORPSE_RECLAIM_RADIUS_LIKE_CPP, CorpseLifecycleCatalogsLikeCpp, ResurrectionOperationCatalogs,
    SPELL_GHOST_LIKE_CPP, SPELL_WISP_SPIRIT_AURA_LIKE_CPP, SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP,
    closest_graveyard_like_cpp, corpse_flags_like_cpp, corpse_item_cache_like_cpp,
    corpse_reclaim_delay_secs_like_cpp, kill_player_like_cpp, player_corpse_like_cpp,
    player_corpse_location_like_cpp, player_in_arena_like_cpp, repop_at_graveyard_like_cpp,
    repop_request_like_cpp, spawn_corpse_bones_like_cpp, tick_player_death_timer,
    updated_death_expire_time_like_cpp,
};
pub use player_release::{build_player_repop_like_cpp, create_corpse_like_cpp};
pub use player_resurrection::{
    reclaim_corpse_like_cpp, resurrect_player_like_cpp, self_resurrect_player_like_cpp,
};
pub use recovery_fences::{RecoveryResumeState, resume_state as resurrection_recovery_resume_state};
pub use player_vitals::{set_player_alive_like_cpp, set_player_ghost_flag_like_cpp};
pub use resurrection_request::{
    continue_player_resurrection_after_teleport, mark_player_resurrection_teleport_completed,
    player_resurrection_continuation_pending, player_resurrection_save_waiting_for_transfer,
    revive_player_for_teleport,
};
pub use state::WorldEntitiesState;
#[cfg(any(test, feature = "test-fixtures"))]
pub use teleport_session::schedule_represented_resurrection_after_teleport;
pub use teleport_session::{
    CLASS_DEATH_KNIGHT_LIKE_CPP, DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP,
    DEATH_KNIGHT_START_MAP_LIKE_CPP, PlayerTeleportHostLikeCpp, TELE_REVIVE_AT_TELEPORT_LIKE_CPP,
    TELE_TO_NONE_LIKE_CPP, TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP, TELE_TO_SPELL_LIKE_CPP,
    TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP, TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP,
    TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP, handle_move_teleport_ack_like_cpp,
    pending_teleport_save_destination_like_cpp,
    process_represented_delayed_resurrection_after_teleport_like_cpp,
    process_represented_delayed_teleport_after_update_like_cpp, teleport_to_with_options,
    teleport_to_with_options_and_resurrection,
};
pub use travel_handlers::{
    TravelHandlerCxLikeCpp, TravelHandlerHostLikeCpp, register_travel_handlers_like_cpp,
};
pub use vehicle_handlers::{
    VehicleHandlerAction, eject_passenger_action_like_cpp,
    move_change_vehicle_seats_action_like_cpp, move_dismiss_vehicle_action_like_cpp,
    request_adjacent_vehicle_seat_action_like_cpp, request_vehicle_exit_action_like_cpp,
    request_vehicle_switch_seat_action_like_cpp, ride_vehicle_interact_action_like_cpp,
};
pub use vehicle_handlers::{
    VehicleHandlerCxLikeCpp, VehicleHandlerHostLikeCpp, register_vehicle_handlers_like_cpp,
};
pub use vehicle_session::{
    VehicleSessionHostLikeCpp, handle_move_change_vehicle_seats, handle_move_dismiss_vehicle,
    handle_request_vehicle_switch_seat, handle_ride_vehicle_interact,
    represented_move_change_vehicle_seats_like_cpp, represented_move_dismiss_vehicle_like_cpp,
    represented_request_vehicle_switch_seat_like_cpp, represented_ride_vehicle_interact_like_cpp,
};
pub use visibility_session::{
    VisibilitySessionHostLikeCpp, apply_deferred_player_visibility_refresh_like_cpp,
    apply_move_init_active_mover_complete_like_cpp, can_see_phase_shift_like_cpp,
    current_seer_guid_like_cpp, dynamic_object_create_data_from_canonical_like_cpp,
    flush_pending_visibility_refresh_with_catalogs_like_cpp,
    force_update_visibility_with_catalogs_like_cpp,
    handle_destroy_visible_object_like_cpp_command_like_cpp,
    represented_gameobject_go_state_for_viewer_like_cpp, represented_player_flags_value_like_cpp,
    represented_player_has_flag_like_cpp, represented_player_phase_shift_like_cpp,
    represented_visibility_source_combat_reach_like_cpp,
    represented_visibility_source_position_like_cpp, resolved_visible_resting_like_cpp,
    send_nearby_creatures_with_catalogs_like_cpp, send_nearby_gameobjects,
    send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp,
    sync_current_player_session_visibility_detection_like_cpp,
    update_visibility_with_catalogs_like_cpp, visible_creatures_from_canonical_map_like_cpp,
    visible_dynamic_objects_from_canonical_map_like_cpp,
    visible_gameobjects_from_canonical_map_like_cpp, visible_other_players_from_registry_like_cpp,
    visible_world_creatures_from_map_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use visibility_session::{
    WAYPOINT_MOTION_TYPE_LIKE_CPP, choose_creature_flags_like_cpp,
    creature_create_movement_flags_like_cpp, creature_create_position_after_hover_offset_like_cpp,
    creature_movement_generator_type_from_db_like_cpp,
    normalize_creature_template_speed_run_like_cpp,
    normalize_creature_template_speed_walk_like_cpp, normalized_creature_wander_distance_like_cpp,
    spawn_difficulties_contains_spawn_mode_like_cpp,
    materialize_creature_spawn_row_for_test,
};
pub use world_entities_host::WorldEntitiesHostLikeCpp;
