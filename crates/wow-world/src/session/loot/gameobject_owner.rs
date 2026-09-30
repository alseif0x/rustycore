//! Gameobject loot state and guarded lifecycle operations.
use super::*;

impl WorldSession {
    pub(crate) fn canonical_gameobject_is_fully_looted_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<bool> {
        self.mutate_canonical_gameobject_by_guid_like_cpp(guid, |gameobject| {
            gameobject.is_fully_looted_like_cpp()
        })
    }
    pub(crate) fn set_canonical_gameobject_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(managed.map_mut().set_gameobject_loot_state_like_cpp(
            guid,
            state,
            unit_guid,
            game_time_secs,
            chest_restock_time_secs,
            shared_loot_is_changed_like_cpp,
        ))
    }
    /// Applies the global fully-looted transition only if the exact authority
    /// generation and pool topology observed by `DoLootRelease` are still
    /// current. The canonical map lock is acquired before the authority lock,
    /// matching personal-loot upsert order and making check+state mutation one
    /// C++-serialized operation.
    pub(crate) fn set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let object_authority = managed
            .map()
            .get_typed_game_object(guid)?
            .loot_authority_like_cpp()
            .clone();
        if !object_authority.shares_storage_like_cpp(authority) {
            return None;
        }

        authority.with_fully_looted_lifecycle_observation_like_cpp(
            object_generation,
            lifecycle_revision,
            || {
                managed.map_mut().set_gameobject_loot_state_like_cpp(
                    guid,
                    state,
                    unit_guid,
                    game_time_secs,
                    chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        )
    }
    /// Detached durable-claim completion may transition the object only when
    /// no client still has any shared or personal loot pool open. The final
    /// viewer check and map mutation are serialized under the authority lock.
    pub(crate) fn set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let object_authority = managed
            .map()
            .get_typed_game_object(guid)?
            .loot_authority_like_cpp()
            .clone();
        if !object_authority.shares_storage_like_cpp(authority) {
            return None;
        }

        authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
            object_generation,
            lifecycle_revision,
            || {
                managed.map_mut().set_gameobject_loot_state_like_cpp(
                    guid,
                    state,
                    unit_guid,
                    game_time_secs,
                    chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        )
    }
    pub(in crate::session) fn represented_gameobject_loot_ids_have_quest_loot_like_cpp(
        &self,
        loot_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let Some(stores) = self.loot_stores.as_ref() else {
            return false;
        };
        let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
            return false;
        };
        loot_ids
            .into_iter()
            .filter(|id| *id != 0)
            .any(|loot_id| store.have_quest_loot_for_like_cpp(loot_id, stores.as_ref()))
    }
    pub(in crate::session) fn represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
        &self,
        loot_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let Some(stores) = self.loot_stores.as_ref() else {
            return false;
        };
        let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
            return false;
        };
        loot_ids.into_iter().filter(|id| *id != 0).any(|loot_id| {
            store.have_quest_loot_for_player_like_cpp(loot_id, stores.as_ref(), |item_id| {
                self.represented_player_has_quest_for_loot_item_like_cpp(item_id)
            })
        })
    }
}
