// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical loot-cache reconciliation and map-owned loot GUID allocation.

use super::*;

impl WorldSession {
    /// Refresh the session-local window from the object-owned source of truth.
    /// The local table remains a packet-building cache only.
    pub(super) fn reconcile_represented_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return false;
        };
        let Some(snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                owner_guid,
                player_guid,
            );
            return false;
        };
        self.cache_represented_owned_loot_snapshot_like_cpp(owner_guid, player_guid, snapshot);
        true
    }

    /// Drops only this session/player's packet-building mirror.
    pub(super) fn discard_represented_personal_loot_cache_for_player_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) {
        self.loot_table.remove(&owner_guid);
        self.represented_loot_cache_generations_like_cpp
            .remove(&owner_guid);
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
    }

    pub(super) fn next_represented_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let canonical = self.next_canonical_loot_object_guid_like_cpp(owner_guid);
        #[cfg(test)]
        {
            canonical.or_else(|| {
                (!owner_guid.is_empty()).then(|| represented_loot_object_guid_like_cpp(owner_guid))
            })
        }
        #[cfg(not(test))]
        {
            canonical
        }
    }

    /// Mirrors `Loot::Loot(Map*)`: every concrete pool receives a fresh
    /// map-owned `HighGuid::LootObject` low GUID.
    pub(super) fn next_canonical_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        (|| {
            let owner_map_id = u32::from(owner_guid.map_id());
            let key = self.canonical_object_lookup_map_key_like_cpp(owner_map_id)?;
            if key.map_id != owner_map_id {
                return None;
            }
            let manager = self.canonical_map_manager.as_ref()?;
            let mut manager = manager.lock().ok()?;
            let map = manager.find_map_mut(key.map_id, key.instance_id)?.map_mut();
            let counter = map.generate_low_guid_like_cpp(HighGuid::LootObject).ok()?;
            let map_id = u16::try_from(key.map_id).ok()?;
            Some(ObjectGuid::create_world_object(
                HighGuid::LootObject,
                0,
                self.realm_id(),
                map_id,
                0,
                0,
                counter,
            ))
        })()
    }

    pub(super) fn refresh_represented_loot_owner_canonical_summary_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, player_guid);
        } else if owner_guid.is_creature_or_vehicle()
            && self
                .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, player_guid)
                .is_none()
        {
            self.loot_table.remove(&owner_guid);
        }
    }

    pub(super) fn record_represented_disenchant_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _spell_id: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::Disenchant {
                player_guid: _player_guid,
                spell_id: _spell_id,
            },
        );
    }
}
