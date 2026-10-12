//! Session instance state and its operations.

mod battleground_handlers;
mod binding;
mod constants;
mod contracts;
mod difficulty;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixture_access;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod group_handlers;
mod handlers;
mod instance;
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
pub use state::InstanceState;
