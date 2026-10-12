//! Session-independent world-entity state and represented creature publication.

mod aggro;
mod catalogs;
mod contracts;
mod creature;
pub mod creature_aggro_contracts;
mod creature_aura_tracking;
mod creature_interaction;
mod creature_kill;
pub mod creature_movement_adapter;
mod creature_publication;
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
mod gameobject_interaction;
mod gameobject_overrides;
mod gameobject_query;
mod gameobject_state;
mod gameobject_use;
mod gameobject_use_dispatch;
mod gameobject_use_kinds;
mod gameobject_use_preamble;
mod gameobject_use_pvp;
mod gameobject_use_scripted;
mod gameobject_use_support;
mod gameobject_use_types;
mod movement_acks;
mod movement_host;
mod movement_opcodes;
mod pending_runtime;
mod player_attack;
mod spawn;
mod spell_click;
mod state;
mod world_entities_host;

pub use aggro::{
    creature_threat_value_on_map_like_cpp, mirror_creature_threat_from_attacker_on_map_like_cpp,
};
pub use catalogs::CreatureSpawnCatalogsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedCreatureKillEventLikeCpp;
pub use contracts::{
    CreatureCreateStatsLikeCpp, MaterializedCreatureSpawnLikeCpp, PendingCreatureKillRewardLikeCpp,
    PendingCreatureSpawn, RepresentedCreatureAuraLikeCpp,
    RepresentedSpellClickCreatureSnapshotLikeCpp,
};
pub use creature::creature_message_to_set_target_allows_like_cpp;
pub use creature_publication::represented_creature_aura_info_like_cpp;
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
pub use gameobject_interaction::{
    represented_gameobject_display_box_contains_like_cpp,
    represented_gameobject_interaction_distance_like_cpp,
};
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
    apply_represented_gameobject_post_use_spell_like_cpp_internal,
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
pub use state::WorldEntitiesState;
pub use world_entities_host::WorldEntitiesHostLikeCpp;
