use super::LootState;
use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

impl LootState {
    /// Retire the detached Rust representation of Loot owned by an Item that
    /// a durable transaction has committed to destroy. C++ gets the same
    /// window teardown from destroying the Item and its owned `Loot`; this is
    /// deliberately narrower than `DoLootReleaseAll` and cannot consume or
    /// otherwise mutate an unrelated active loot owner.
    pub fn retire_committed_destroyed_item_loot_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if self.active_loot_view_owners.contains(&item_guid) || self.is_active_loot_guid(item_guid)
        {
            self.close_stale_active_loot_view_like_cpp(hub, item_guid, player_guid);
        }
        self.loot_table.remove(&item_guid);
    }
}
