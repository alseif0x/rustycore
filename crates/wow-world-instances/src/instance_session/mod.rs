// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World state and instance admission at the session boundary (#1363 slice
//! 11b; moved from the World `session/world_state/operations.rs`,
//! `session/instances/{map_key,binding,instance}.rs`,
//! `session/rest_progression.rs` and `session/map_admission.rs`; with #1363
//! slice 13a, the map entry from `session/lifecycle/map_entry.rs`).
//!
//! C++ `Player::UpdateZone`/`UpdateArea` with the RestMgr flags and the
//! hostility update, `Player::IsInAreaTriggerRadius`, the area-trigger and
//! tavern transitions, `Player::CheckAreaExploreAndOutdoor`,
//! `Map::PlayerCannotEnter`, the `MapManager::CreateMap` player context and
//! side effects, and the login and far-transfer map entry. Each host method
//! below is the same-named `WorldSession` operation (or field), called at the
//! exact point the body called it before the move; the former `#[cfg(test)]`
//! fixture steps run only when `SessionHubHostLikeCpp::test_fixtures_ref_like_cpp`
//! is `Some`.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_data::{AreaTriggerStore, TavernAreaTriggerStoreLikeCpp};
use wow_world_core::session::{HubMut, HubRef};
use wow_world_entities::PlayerTeleportHostLikeCpp;
use wow_world_spell::AuraHostLikeCpp;

use crate::InstanceState;

mod access_requirement;
mod activate_taxi;
mod admission;
mod area_trigger_opcode;
mod area_triggers;
mod create_map;
mod exploration;
mod login_load;
mod map_entry;
mod rest_flags;
mod zone_area;

pub use access_requirement::{
    access_requirement_abort_like_cpp, access_requirement_leader_has_achievement_like_cpp,
};
pub use activate_taxi::handle_activate_taxi;
#[cfg(any(test, feature = "test-fixtures"))]
pub use activate_taxi::represented_taxi_usable_mount_displays_like_cpp;
pub use admission::{
    create_map_active_instance_lock_context_like_cpp, create_map_instance_owner_guid_like_cpp,
    create_map_player_context_like_cpp, player_cannot_enter_target_map_like_cpp,
};
pub use area_trigger_opcode::{AreaTriggerHostLikeCpp, handle_area_trigger_with_catalogs_like_cpp};
pub use area_triggers::{
    check_area_triggers_with_catalogs_like_cpp,
    handle_represented_tavern_area_trigger_with_catalog_like_cpp,
    player_is_in_area_trigger_radius_like_cpp,
    revalidate_represented_tavern_resting_with_catalog_like_cpp,
};
pub use create_map::{
    CreateMapSideEffectApplySummaryLikeCpp, apply_create_map_side_effects_like_cpp,
};
pub use exploration::{
    check_area_explore_and_outdoor_represented_with_catalogs_like_cpp,
    player_explored_zones_snapshot_like_cpp,
};
pub use login_load::{
    load_represented_explored_zones_like_cpp, load_represented_player_difficulties_like_cpp,
};
pub use map_entry::{
    ensure_canonical_player_owner_for_map_like_cpp,
    ensure_canonical_world_map_for_current_player_like_cpp, prepare_canonical_map_entry_like_cpp,
    try_attach_worldport_destination_like_cpp,
    try_attach_worldport_destination_with_publication_like_cpp,
};
pub use rest_flags::{
    remove_player_rest_flag_like_cpp, set_player_rest_flag_like_cpp,
    take_player_deferred_rest_flag_update_dirty_like_cpp, with_owned_player_mut_for_rest_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use zone_area::{
    set_represented_tavern_resting_like_cpp, update_area_represented_with_rest_update_like_cpp,
};
pub use zone_area::{
    update_zone_represented_like_cpp, update_zone_represented_without_rest_update_packet_like_cpp,
};

pub const REST_FLAG_IN_TAVERN_LIKE_CPP: u32 = 0x1;
pub const REST_FLAG_IN_CITY_LIKE_CPP: u32 = 0x2;
pub const REST_FLAG_IN_FACTION_AREA_LIKE_CPP: u32 = 0x4;
pub const AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP: u32 = 0x0000_0010;
const AREA_FLAG_CONTESTED_LIKE_CPP: u32 = 0x0004_0000;
const AREA_FLAG_COMBAT_ZONE_LIKE_CPP: u32 = 0x0100_0000;
pub const TRANSFER_ABORT_ERROR_LIKE_CPP: u32 = 1;
pub const TRANSFER_ABORT_DIFFICULTY_LIKE_CPP: u32 = 8;
pub const TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP: u32 = 2;
pub const TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP: u32 = 4;
pub const TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP: u32 = 6;
pub const TRANSFER_ABORT_NEED_GROUP_LIKE_CPP: u32 = 11;

/// The process-owned area-trigger catalogs the area-trigger transition
/// reads (the World `AreaTriggerCatalogsLikeCpp` fields of the same names).
pub trait AreaTriggerCatalogAccessLikeCpp: Sync {
    fn destinations(&self) -> &Arc<AreaTriggerStore>;
    fn taverns(&self) -> &Arc<TavernAreaTriggerStoreLikeCpp>;
    /// The `db2` and `scripts` fields the area-trigger opcode reads (#1363 slice 4d).
    fn db2(&self) -> &Arc<wow_data::AreaTriggerDb2Store>;
    fn scripts(&self) -> &Arc<wow_data::AreaTriggerScriptStoreLikeCpp>;
}

/// The World operations the moved world-state and instance-admission bodies
/// still call; each is the same-named `WorldSession` operation, or the named
/// field or split.
pub trait InstanceSessionHostLikeCpp: PlayerTeleportHostLikeCpp + AuraHostLikeCpp {
    /// `crate::session::split_instances_ref`.
    fn split_instances_ref(&self) -> (&InstanceState, HubRef<'_>);
    /// `crate::session::split_instances_mut`.
    fn split_instances_mut(&mut self) -> (&mut InstanceState, HubMut<'_>);
    /// The session view's `is_pvp_realm_like_cpp` field.
    fn is_pvp_realm_like_cpp(&self) -> bool;
    /// `WorldSession::mutate_player_rest_state_like_cpp` (World test build; called
    /// only when `test_fixtures_ref_like_cpp` is `Some`, so never this default).
    #[cfg(any(test, feature = "test-fixtures"))]
    fn mutate_player_rest_state_like_cpp<R>(
        &mut self,
        _f: impl FnOnce(&mut wow_entities::PlayerRestState) -> R,
    ) -> Option<R> {
        None
    }
    fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool;
    /// The inventory item counts the access-requirement item check reads (the
    /// World split forward of that name, retired in #1363 slice 12d).
    fn represented_inventory_item_counts_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<u32, u32>>;
    /// Whether the World player meets one `PlayerCondition` row (the
    /// map-difficulty condition closure of the moved access-requirement step).
    fn represented_player_meets_condition_like_cpp(
        &self,
        condition: &wow_data::PlayerConditionEntry,
    ) -> bool;
    fn access_requirement_abort_like_cpp(
        &self,
        map_id: u32,
        requested_difficulty: u8,
    ) -> Option<(u32, u8, i32)>;
    fn current_player_is_in_raid_group_like_cpp(&self) -> bool;
    fn authoritative_group_membership_like_cpp(&self) -> Option<u64>;
    fn player_loading(&self) -> Option<ObjectGuid>;
    fn ensure_canonical_player_owner_exists_like_cpp(&mut self, key: wow_map::MapKey) -> bool;
}
