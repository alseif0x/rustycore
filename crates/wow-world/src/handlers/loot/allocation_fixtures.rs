//! Allocation fixtures use the same map and item generators as durable loot grants.

use super::*;

pub fn gameobject_loot_group_state_for_test(session: &WorldSession, use_group_rules: bool, player: ObjectGuid) -> (u8, ObjectGuid, ObjectGuid) {
    session.represented_gameobject_chest_group_state_like_cpp(use_group_rules, player)
}

pub fn allocate_loot_guid_for_test(session: &mut WorldSession, owner: ObjectGuid) -> Option<ObjectGuid> {
    session.next_canonical_loot_object_guid_like_cpp(owner)
}

pub fn materialize_loot_pools_for_test(
    session: &mut WorldSession,
    owner: ObjectGuid,
    player: ObjectGuid,
    loot: CreatureLoot,
    personal: bool,
) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)> {
    session.represented_loot_authority_pools_like_cpp(owner, player, loot, personal)
}

pub fn allocate_loot_item_guids_for_test(
    session: &WorldSession,
    count: usize,
) -> Option<Vec<(u64, ObjectGuid)>> {
    session.allocate_installed_loot_item_guids_for_test(count)
}
