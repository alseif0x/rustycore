//! Canonical allocation setup and observations for loot application tests.

use super::*;

pub use crate::handlers::loot::{
    gameobject_loot_group_state_for_test,
    allocate_loot_guid_for_test, allocate_loot_item_guids_for_test,
    materialize_loot_pools_for_test,
};

pub fn attach_loot_allocator_for_test(session: &mut WorldSession, owner_guid: ObjectGuid) {
    let kind = if owner_guid.is_game_object() {
        AccessorObjectKind::GameObject
    } else {
        AccessorObjectKind::Creature
    };
    attach_canonical_map_object_for_loot_test(
        session,
        kind,
        canonical_world_object_for_loot_test(owner_guid, u32::from(owner_guid.map_id()), Position::ZERO),
    );
}

pub fn next_map_loot_guid_for_test(session: &WorldSession, map_id: u32, instance_id: u32) -> Option<i64> {
    let manager = session.canonical_map_manager.as_ref()?;
    let mut manager = manager.lock().ok()?;
    manager.find_map_mut(map_id, instance_id)?.map_mut().get_max_low_guid_like_cpp(HighGuid::LootObject).ok()
}
