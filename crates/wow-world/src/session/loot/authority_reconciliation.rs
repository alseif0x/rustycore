//! Existing multiowner authority reads/rebinds, retained until handoff.
use super::*;

impl WorldSession {
    pub(crate) fn read_legacy_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        self.read_legacy_creature_loot_authority_on_map_like_cpp(
            guid,
            wow_map::MapKey::new(u32::from(map_id), instance_id),
        )
    }
    pub(crate) fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = self.map_manager.as_ref()?;
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(map_id, map_key.instance_id, guid)
            .map(|world_creature| world_creature.creature.loot_authority_like_cpp().clone())
    }
    pub(crate) fn rebind_legacy_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        self.rebind_legacy_creature_loot_authority_on_map_like_cpp(
            guid,
            wow_map::MapKey::new(u32::from(map_id), instance_id),
            expected,
            expected_stamp,
            authority,
        )
    }
    pub(crate) fn rebind_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = self.map_manager.as_ref()?;
        manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature_mut(map_id, map_key.instance_id, guid)
            .and_then(|world_creature| {
                world_creature
                    .creature
                    .rebind_loot_authority_if_current_like_cpp(expected, expected_stamp, authority)
            })
    }
    pub(crate) fn read_canonical_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        self.read_canonical_creature_loot_authority_on_map_like_cpp(guid, map_key)
    }
    pub(crate) fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
    }
    pub(crate) fn rebind_canonical_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        self.rebind_canonical_creature_loot_authority_on_map_like_cpp(
            guid,
            map_key,
            expected,
            expected_stamp,
            authority,
        )
    }
    pub(crate) fn rebind_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let manager = self.canonical_map_manager.as_ref()?;
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
    pub(crate) fn read_canonical_gameobject_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        self.read_canonical_gameobject_loot_authority_on_map_like_cpp(guid, map_key)
    }
    pub(crate) fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .get_typed_game_object(guid)
            .map(|gameobject| gameobject.loot_authority_like_cpp().clone())
    }
    pub(crate) fn rebind_canonical_gameobject_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
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
}
