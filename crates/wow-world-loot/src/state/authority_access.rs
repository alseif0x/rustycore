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
        hub.core
            .loot_release_access_like_cpp()
            .read_legacy_creature_loot_authority_on_map_like_cpp(guid, map_key)
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
        hub.core
            .loot_release_access_like_cpp()
            .read_canonical_creature_loot_authority_on_map_like_cpp(guid, map_key)
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
        hub.core
            .loot_release_access_like_cpp()
            .rebind_canonical_creature_loot_authority_on_map_like_cpp(
                guid,
                map_key,
                expected,
                expected_stamp,
                authority,
            )
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
        hub.core
            .loot_release_access_like_cpp()
            .read_canonical_gameobject_loot_authority_on_map_like_cpp(guid, map_key)
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

    /// F6-7 R7b-2a: the guarded creature loot-lifecycle mutation now routes
    /// through the session's one gated mutation root
    /// (`SessionCore::mutate_world_creature_if_fully_looted_observation_like_cpp`),
    /// which admits the representation against the current canonical
    /// incarnation, takes the canonical lock before the legacy one, runs the
    /// mutation once, applies it to the incarnation and only then synchronizes
    /// the legacy representation. The two-store body that used to live here is
    /// gone rather than duplicated.
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
        hub.core
            .mutate_world_creature_if_fully_looted_observation_like_cpp(
                guid,
                authority,
                object_generation,
                lifecycle_revision,
                f,
            )
    }

    /// Detached durable-claim completion variant of the guarded creature
    /// mutation. It additionally requires every authoritative loot viewer set
    /// to remain empty through the map mutation. F6-7 R7b-2a: same single gated
    /// root as the viewed variant.
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
        hub.core
            .mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
                guid,
                authority,
                object_generation,
                lifecycle_revision,
                f,
            )
    }
}
