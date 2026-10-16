// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map-owned identities and canonical admission for Player-origin casts
//! (#1363 slice 12b7), moved from the World `session/player_cast/identity.rs`.

#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::atomic::{AtomicI64, Ordering};

use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_world_core::session::SessionHubHostLikeCpp;

use crate::spell_cast_adapter::represented_spell_cast_guid_for_map_like_cpp;

// Only Session-only unit fixtures lack a canonical Map. Production allocation
// always consumes the admitted Map's existing Cast sequence. The counter is
// read only when the World test fixtures are lent (the World test build).
#[cfg(any(test, feature = "test-fixtures"))]
static NEXT_REPRESENTED_SPELL_CAST_COUNTER_LIKE_CPP: AtomicI64 = AtomicI64::new(1);

/// Represented `Spell::m_castId` generation.
///
/// C++ creates a `HighGuid::Cast` with `SPELL_CAST_SOURCE_NORMAL`, map id,
/// spell id, and `Map::GenerateLowGuid<HighGuid::Cast>()`. Admission and
/// allocation happen under one guard; the result is not a lease across await.
pub fn next_represented_spell_cast_guid_like_cpp<H: SessionHubHostLikeCpp + ?Sized>(
    host: &H,
    spell_id: i32,
) -> Option<ObjectGuid> {
    allocate_player_cast_identity_like_cpp(host, spell_id).map(|(guid, _)| guid)
}

pub fn allocate_player_cast_identity_like_cpp<H: SessionHubHostLikeCpp + ?Sized>(
    host: &H,
    spell_id: i32,
) -> Option<(ObjectGuid, Option<u64>)> {
    u32::try_from(spell_id).ok().filter(|id| *id != 0)?;
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && host.core().player_handle_like_cpp.is_none()
        && host.core().canonical_map_manager.is_none()
    {
        return Some((
            represented_spell_cast_guid_for_map_like_cpp(
                host.core().realm_id(),
                host.core().player_map_id_like_cpp(),
                spell_id,
                NEXT_REPRESENTED_SPELL_CAST_COUNTER_LIKE_CPP.fetch_add(1, Ordering::Relaxed),
            ),
            None,
        ));
    }
    let handle = host.core().player_handle_like_cpp?;
    if Some(handle.guid()) != host.player_guid() {
        return None;
    }
    let mut manager = host.core().canonical_map_manager.as_ref()?.lock().ok()?;
    let (key, revision) = manager.player_active_residence_revision_like_cpp(handle)?;
    let map_id = u16::try_from(key.map_id).ok()?;
    let counter = manager
        .find_map_mut(key.map_id, key.instance_id)?
        .map_mut()
        .generate_low_guid_like_cpp(HighGuid::Cast)
        .ok()?;
    Some((
        represented_spell_cast_guid_for_map_like_cpp(
            host.core().realm_id(),
            map_id,
            spell_id,
            counter,
        ),
        Some(revision),
    ))
}
