//! Session instance state and its operations.

mod battleground_handlers;
mod binding;
mod constants;
mod contracts;
mod difficulty;
mod dungeon_finding_handlers;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixture_access;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod group_handlers;
mod handlers;
mod instance;
mod instance_session;
mod map_key;
mod map_resolution;
mod operations;
mod pending_raid_lock;
mod state;

pub use battleground_handlers::{
    BattlegroundHandlerCxLikeCpp, BattlegroundHandlerHostLikeCpp,
    register_battleground_handlers_like_cpp,
};
pub use binding::create_map_instance_lock_token_like_cpp;
pub use constants::{
    DIFFICULTY_10_N_LIKE_CPP, DIFFICULTY_NORMAL_LIKE_CPP, DIFFICULTY_NORMAL_RAID_LIKE_CPP,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedAreaZoneCriteriaLikeCpp;
pub use contracts::{RepresentedAdventureMapStartQuestLikeCpp, RepresentedPendingBind};
pub use difficulty::SessionDifficultyKindLikeCpp;
pub use dungeon_finding_handlers::{
    DungeonFindingHandlerCxLikeCpp, DungeonFindingHandlerHostLikeCpp,
    register_dungeon_finding_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use fixtures::InstanceTestFixtureLikeCpp;
pub use group_handlers::GroupPublicationTailLikeCpp;
pub use group_handlers::resolved_group_guid_like_cpp;
pub use group_handlers::{
    GroupHandlerCxLikeCpp, GroupHandlerHostLikeCpp, register_group_handlers_like_cpp,
};
pub use handlers::{
    InstanceDifficultyHandlerCxLikeCpp, InstanceLockOperationsHandlerCxLikeCpp,
    InstanceLockResponseOutcomeLikeCpp, InstanceRaidInfoHandlerCxLikeCpp,
    InstanceResetMethodLikeCpp, InstancesHandlerHostLikeCpp,
    handle_instance_lock_response_like_cpp, handle_request_raid_info_like_cpp,
    handle_reset_instances_like_cpp, handle_set_difficulty_id_like_cpp,
    handle_set_dungeon_difficulty_like_cpp, handle_set_raid_difficulty_like_cpp,
    handle_set_saved_instance_extend_like_cpp, handle_toggle_difficulty_like_cpp,
    register_instance_handlers_like_cpp, reset_represented_instances_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use instance_session::{
    AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP, set_represented_tavern_resting_like_cpp,
    update_area_represented_with_rest_update_like_cpp,
};
pub use instance_session::{
    AreaTriggerCatalogAccessLikeCpp, CreateMapSideEffectApplySummaryLikeCpp,
    InstanceSessionHostLikeCpp, REST_FLAG_IN_CITY_LIKE_CPP, REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
    REST_FLAG_IN_TAVERN_LIKE_CPP, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
    TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP,
    TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP,
    apply_create_map_side_effects_like_cpp,
    check_area_explore_and_outdoor_represented_with_catalogs_like_cpp,
    check_area_triggers_with_catalogs_like_cpp, create_map_active_instance_lock_context_like_cpp,
    create_map_instance_owner_guid_like_cpp, create_map_player_context_like_cpp,
    ensure_canonical_player_owner_for_map_like_cpp,
    ensure_canonical_world_map_for_current_player_like_cpp,
    handle_represented_tavern_area_trigger_with_catalog_like_cpp,
    load_represented_explored_zones_like_cpp, load_represented_player_difficulties_like_cpp,
    player_cannot_enter_target_map_like_cpp, player_explored_zones_snapshot_like_cpp,
    player_is_in_area_trigger_radius_like_cpp, prepare_canonical_map_entry_like_cpp,
    try_attach_worldport_destination_like_cpp,
    try_attach_worldport_destination_with_publication_like_cpp, update_zone_represented_like_cpp,
    update_zone_represented_without_rest_update_packet_like_cpp,
};
pub use state::InstanceState;
