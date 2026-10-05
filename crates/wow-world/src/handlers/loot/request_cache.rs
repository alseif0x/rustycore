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
            self.loot
                .discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            return false;
        };
        self.loot
            .cache_represented_owned_loot_snapshot_like_cpp(owner_guid, player_guid, snapshot);
        true
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
        self.core
            .loot_release_owner_access_like_cpp()
            .next_canonical_loot_object_guid_like_cpp(owner_guid)
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
            self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);
        }
    }
}
