use super::LootState;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;
use wow_world_core::session::HubMut;

impl LootState {
    pub fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32 {
        wow_loot::generate_money_loot_with_rate_like_cpp(
            min_amount,
            max_amount,
            rate,
            &mut hub.core.driver.represented_runtime_rng_like_cpp,
        )
    }

    pub fn set_canonical_gameobject_loot_state_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
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
    pub fn set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
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
    pub fn set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
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
}
