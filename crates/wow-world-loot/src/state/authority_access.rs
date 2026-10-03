use super::LootState;
use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityStamp};
use wow_world_core::session::{HubMut, HubRef};

impl LootState {
    pub fn read_legacy_creature_loot_authority_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        self.read_legacy_creature_loot_authority_on_map_like_cpp(
            hub,
            guid,
            wow_map::MapKey::new(u32::from(map_id), instance_id),
        )
    }

    pub fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = hub.core.map_manager.as_ref()?;
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(map_id, map_key.instance_id, guid)
            .map(|world_creature| world_creature.creature.loot_authority_like_cpp().clone())
    }

    pub fn read_canonical_creature_loot_authority_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        self.read_canonical_creature_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }

    pub fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
    }

    pub fn rebind_canonical_creature_loot_authority_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        self.rebind_canonical_creature_loot_authority_on_map_like_cpp(
            hub,
            guid,
            map_key,
            expected,
            expected_stamp,
            authority,
        )
    }

    pub fn rebind_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager
            .find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut()
            .get_typed_creature_mut(guid)
            .and_then(|creature| {
                creature.rebind_loot_authority_if_current_like_cpp(
                    expected,
                    expected_stamp,
                    authority,
                )
            })
    }

    pub fn read_canonical_gameobject_loot_authority_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        self.read_canonical_gameobject_loot_authority_on_map_like_cpp(hub, guid, map_key)
    }

    pub fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .get_typed_game_object(guid)
            .map(|gameobject| gameobject.loot_authority_like_cpp().clone())
    }

    pub fn rebind_canonical_gameobject_loot_authority_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager
            .find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut()
            .get_typed_game_object_mut(guid)
            .and_then(|gameobject| {
                gameobject.rebind_loot_authority_if_current_like_cpp(
                    expected,
                    expected_stamp,
                    authority,
                )
            })
    }

    pub fn mutate_world_creature_if_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut wow_world_core::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        let manager = hub.core.map_manager.as_ref().cloned()?;
        let guarded_result = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = manager.find_creature_mut(map_id, instance_id, guid)?;
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || {
                    let result = f(creature);
                    (result, creature.creature.clone())
                },
            )
        }?;
        let (result, creature) = guarded_result;
        hub.core.sync_canonical_creature_entity_like_cpp(creature);
        Some(result)
    }

    /// Detached durable-claim completion variant of the guarded creature
    /// mutation. It additionally requires every authoritative loot viewer set
    /// to remain empty through the map mutation.
    pub fn mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut wow_world_core::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        let manager = hub.core.map_manager.as_ref().cloned()?;
        let guarded_result = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = manager.find_creature_mut(map_id, instance_id, guid)?;
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || {
                    let result = f(creature);
                    (result, creature.creature.clone())
                },
            )
        }?;
        let (result, creature) = guarded_result;
        hub.core.sync_canonical_creature_entity_like_cpp(creature);
        Some(result)
    }
}
