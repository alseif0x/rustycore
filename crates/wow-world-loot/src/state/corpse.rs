use super::LootState;
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;
use wow_world_core::session::HubMut;

impl LootState {
    pub fn remove_canonical_corpse_lootable_dynamic_flag_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        corpse_guid: ObjectGuid,
    ) -> bool {
        hub.core
            .loot_release_access_like_cpp()
            .remove_canonical_corpse_lootable_dynamic_flag_like_cpp(corpse_guid)
    }

    pub fn remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        corpse_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
    ) -> bool {
        hub.core.loot_release_access_like_cpp().remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(corpse_guid, authority, object_generation, lifecycle_revision)
    }
}
