// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical loot-cache reconciliation and map-owned loot GUID allocation
//! (#1363 slice 5a; moved from the World `handlers/loot/request_cache.rs`).

use super::*;

/// Refresh the session-local window from the object-owned source of truth.
/// The local table remains a packet-building cache only.
pub fn reconcile_represented_loot_cache_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    // The admitted application owner holds this transition; the World
    // shell only builds the borrowed loot-release context (#1263 F6).
    host.loot_release_cx_like_cpp()
        .reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid)
}

pub fn next_represented_loot_object_guid_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
) -> Option<ObjectGuid> {
    let canonical = next_canonical_loot_object_guid_like_cpp(host, owner_guid);
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_like_cpp().is_some() {
        return canonical.or_else(|| {
            (!owner_guid.is_empty()).then(|| represented_loot_object_guid_like_cpp(owner_guid))
        });
    }
    canonical
}

/// Mirrors `Loot::Loot(Map*)`: every concrete pool receives a fresh
/// map-owned `HighGuid::LootObject` low GUID.
pub fn next_canonical_loot_object_guid_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
) -> Option<ObjectGuid> {
    host.core_mut()
        .loot_release_owner_access_like_cpp()
        .next_canonical_loot_object_guid_like_cpp(owner_guid)
}

pub fn refresh_represented_loot_owner_canonical_summary_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) {
    if owner_guid.is_game_object() {
        let _ =
            sync_represented_gameobject_loot_to_canonical_like_cpp(host, owner_guid, player_guid);
    } else if owner_guid.is_creature_or_vehicle()
        && sync_represented_creature_loot_to_canonical_like_cpp(host, owner_guid, player_guid)
            .is_none()
    {
        host.loot_mut()
            .remove_cached_loot_for_owner_like_cpp(owner_guid);
    }
}
