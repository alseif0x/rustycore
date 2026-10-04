// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityStamp};
use wow_map::MapKey;
use super::{LootReleaseAccessLikeCpp, LootReleaseOwnerAccessLikeCpp};

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn next_canonical_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        (|| {
            let owner_map_id = u32::from(owner_guid.map_id());
            let key = self.core.canonical_object_lookup_map_key_like_cpp(owner_map_id)?;
            if key.map_id != owner_map_id {
                return None;
            }
            let manager = self.core.canonical_map_manager.as_ref()?;
            let mut manager = manager.lock().ok()?;
            let map = manager.find_map_mut(key.map_id, key.instance_id)?.map_mut();
            let counter = map.generate_low_guid_like_cpp(wow_core::HighGuid::LootObject).ok()?;
            let map_id = u16::try_from(key.map_id).ok()?;
            Some(ObjectGuid::create_world_object(
                wow_core::HighGuid::LootObject, 0, self.core.realm_id(), map_id, 0, 0, counter,
            ))
        })()
    }

    pub fn represented_owned_loot_authority_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        if owner_guid.is_creature_or_vehicle() {
            // The legacy and canonical maps deliberately use separate locks.
            // Reconcile optimistically with object-local compare/exchange;
            // blind rebinding can otherwise clobber a newer respawn between
            // the read and write phases.
            for _ in 0..8 {
                let canonical_player_map_key =
                    self.core.current_canonical_player_map_key_like_cpp();
                let map_key = canonical_player_map_key
                    .or_else(|| {
                        self.core
                            .canonical_object_lookup_map_key_like_cpp(u32::from(
                                self.core.player_map_id_like_cpp(),
                            ))
                    })
                    .unwrap_or_else(|| {
                        let (map_id, instance_id) =
                            self.core.current_legacy_runtime_map_key_like_cpp();
                        wow_map::MapKey::new(u32::from(map_id), instance_id)
                    });
                let map_key_still_valid = |session: &Self| {
                    session.transitions_like_cpp().loot_reconciliation_map_key_still_valid_like_cpp(
                        map_key,
                        canonical_player_map_key.is_some(),
                    )
                };
                let legacy =
                    self.transitions_like_cpp().read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let canonical = self.transitions_like_cpp()
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let (legacy, canonical) = match (legacy, canonical) {
                    (Some(legacy), Some(canonical)) => (legacy, canonical),
                    (None, None) => return None,
                    (Some(authority), None) | (None, Some(authority)) => {
                        if !map_key_still_valid(self) {
                            continue;
                        }
                        return Some(authority);
                    }
                };
                if !map_key_still_valid(self) {
                    continue;
                }

                let legacy_stamp = legacy.stamp_like_cpp();
                let canonical_stamp = canonical.stamp_like_cpp();
                let selected = crate::session::reconcile_creature_loot_authority_mirrors_like_cpp(
                    &canonical,
                    canonical_stamp,
                    &legacy,
                    legacy_stamp,
                );
                if !map_key_still_valid(self) {
                    continue;
                }
                if self.transitions_like_cpp()
                    .rebind_canonical_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &canonical,
                        canonical_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .core
                    .rebind_legacy_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &legacy,
                        legacy_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }

                if !map_key_still_valid(self) {
                    continue;
                }
                let converged_legacy = self.transitions_like_cpp()
                    .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                let converged_canonical = self.transitions_like_cpp()
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                if converged_legacy && converged_canonical {
                    return Some(selected);
                }
            }

            // Continuous concurrent replacement is safer as a failed request
            // than as an overwrite of the newest mirror.
            return None;
        }

        if owner_guid.is_game_object() {
            let canonical_player_map_key = self.core.current_canonical_player_map_key_like_cpp();
            let map_key = canonical_player_map_key.or_else(|| {
                self.core
                    .canonical_object_lookup_map_key_like_cpp(u32::from(
                        self.core.player_map_id_like_cpp(),
                    ))
            })?;
            let authority =
                self.transitions_like_cpp().read_canonical_gameobject_loot_authority_on_map_like_cpp(owner_guid, map_key)?;
            let still_valid = self.transitions_like_cpp().loot_reconciliation_map_key_still_valid_like_cpp(
                map_key,
                canonical_player_map_key.is_some(),
            );
            return still_valid.then_some(authority);
        }

        None
    }


}

impl LootReleaseAccessLikeCpp<'_> {
    pub fn rebind_canonical_gameobject_loot_authority_like_cpp(
        &self, guid: ObjectGuid, expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp, authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = self.core.canonical_object_lookup_map_key_like_cpp(u32::from(
            self.core.player_map_id_like_cpp(),
        ))?;
        let manager = self.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager.find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut().get_typed_game_object_mut(guid)
            .and_then(|gameobject| {
                gameobject.rebind_loot_authority_if_current_like_cpp(expected, expected_stamp, authority)
            })
    }

    pub fn rebind_canonical_creature_loot_authority_like_cpp(
        &self, guid: ObjectGuid, expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp, authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = self.core.canonical_object_lookup_map_key_like_cpp(u32::from(
            self.core.player_map_id_like_cpp(),
        ))?;
        self.rebind_canonical_creature_loot_authority_on_map_like_cpp(
            guid, map_key, expected, expected_stamp, authority,
        )
    }

    pub fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = self.core.map_manager.as_ref()?;
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(map_id, map_key.instance_id, guid)
            .map(|world_creature| world_creature.creature.loot_authority_like_cpp().clone())
    }

    pub fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = self.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
    }

    pub fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = self.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .get_typed_game_object(guid)
            .map(|gameobject| gameobject.loot_authority_like_cpp().clone())
    }

    pub fn rebind_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let manager = self.core.canonical_map_manager.as_ref()?;
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

    pub fn loot_reconciliation_map_key_still_valid_like_cpp(
        &self,
        map_key: MapKey,
        canonical_player_was_present: bool,
    ) -> bool {
        if canonical_player_was_present {
            return self.core.current_canonical_player_map_key_like_cpp() == Some(map_key);
        }
        if self.core.canonical_map_manager.is_some() {
            return self.core
                .canonical_object_lookup_map_key_like_cpp(map_key.map_id)
                == Some(map_key);
        }
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        u32::from(map_id) == map_key.map_id && instance_id == map_key.instance_id
    }
}
