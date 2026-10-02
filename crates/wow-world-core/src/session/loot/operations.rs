use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityStamp};

impl SessionCore {
    pub fn rebind_legacy_creature_loot_authority_like_cpp(
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

    pub fn rebind_legacy_creature_loot_authority_on_map_like_cpp(
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
}
