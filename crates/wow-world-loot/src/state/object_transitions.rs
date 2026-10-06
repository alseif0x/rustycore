use super::LootState;
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
        hub.core
            .loot_release_access_like_cpp()
            .set_canonical_gameobject_loot_state_like_cpp(
                guid,
                state,
                unit_guid,
                chest_restock_time_secs,
                shared_loot_is_changed_like_cpp,
            )
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
        hub.core
            .loot_release_access_like_cpp()
            .set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                guid,
                authority,
                object_generation,
                lifecycle_revision,
                state,
                unit_guid,
                chest_restock_time_secs,
                shared_loot_is_changed_like_cpp,
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
        hub.core
            .loot_release_access_like_cpp()
            .set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
                guid,
                authority,
                object_generation,
                lifecycle_revision,
                state,
                unit_guid,
                chest_restock_time_secs,
                shared_loot_is_changed_like_cpp,
            )
    }
}
