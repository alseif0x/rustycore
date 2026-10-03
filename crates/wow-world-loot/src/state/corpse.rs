use super::LootState;
use wow_core::ObjectGuid;
use wow_entities::CORPSE_DYNFLAG_LOOTABLE;
use wow_loot::OwnedLootAuthority;
use wow_world_core::session::HubMut;

impl LootState {
    pub fn remove_canonical_corpse_lootable_dynamic_flag_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        corpse_guid: ObjectGuid,
    ) -> bool {
        let Some(map_key) = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(hub.core.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        let Some(corpse) = map.map_mut().get_typed_corpse_mut(corpse_guid) else {
            return false;
        };

        corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
        true
    }

    pub fn remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        corpse_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
    ) -> bool {
        let Some(map_key) = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(hub.core.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        let Some(corpse) = map.map_mut().get_typed_corpse_mut(corpse_guid) else {
            return false;
        };

        authority
            .with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE),
            )
            .is_some()
    }
}
