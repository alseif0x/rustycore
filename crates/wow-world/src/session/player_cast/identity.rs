//! Map-owned identities and canonical admission for Player-origin casts.

use super::*;

// Only Session-only unit fixtures lack a canonical Map. Production allocation
// always consumes the admitted Map's existing Cast sequence.
#[cfg(test)]
static NEXT_REPRESENTED_SPELL_CAST_COUNTER_LIKE_CPP: AtomicI64 = AtomicI64::new(1);

impl WorldSession {
    /// Represented `Spell::m_castId` generation.
    ///
    /// C++ creates a `HighGuid::Cast` with `SPELL_CAST_SOURCE_NORMAL`, map id,
    /// spell id, and `Map::GenerateLowGuid<HighGuid::Cast>()`. Admission and
    /// allocation happen under one guard; the result is not a lease across await.
    pub(crate) fn next_represented_spell_cast_guid_like_cpp(
        &self,
        spell_id: i32,
    ) -> Option<ObjectGuid> {
        self.allocate_player_cast_identity_like_cpp(spell_id)
            .map(|(guid, _)| guid)
    }

    /// The admitted residence revision for the logged-in player.
    ///
    /// A prepared cast is fenced against residence reentry by stamping this
    /// value. Server-triggered timed casts need the same fence as normal
    /// client requests: without it a cast prepared before a map transfer would
    /// still launch after the player returns.
    pub(crate) fn current_player_residence_revision_like_cpp(&self) -> Option<u64> {
        let handle = self.player_handle_like_cpp?;
        if Some(handle.guid()) != self.player_guid() {
            return None;
        }
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .player_active_residence_revision_like_cpp(handle)
            .map(|(_, revision)| revision)
    }

    pub(in crate::session) fn allocate_player_cast_identity_like_cpp(
        &self,
        spell_id: i32,
    ) -> Option<(ObjectGuid, Option<u64>)> {
        u32::try_from(spell_id).ok().filter(|id| *id != 0)?;
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() && self.canonical_map_manager.is_none() {
            return Some((
                represented_spell_cast_guid_for_map_like_cpp(
                    self.realm_id(),
                    self.player_map_id_like_cpp(),
                    spell_id,
                    NEXT_REPRESENTED_SPELL_CAST_COUNTER_LIKE_CPP.fetch_add(1, Ordering::Relaxed),
                ),
                None,
            ));
        }
        let handle = self.player_handle_like_cpp?;
        if Some(handle.guid()) != self.player_guid() {
            return None;
        }
        let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let (key, revision) = manager.player_active_residence_revision_like_cpp(handle)?;
        let map_id = u16::try_from(key.map_id).ok()?;
        let counter = manager
            .find_map_mut(key.map_id, key.instance_id)?
            .map_mut()
            .generate_low_guid_like_cpp(HighGuid::Cast)
            .ok()?;
        Some((
            represented_spell_cast_guid_for_map_like_cpp(
                self.realm_id(),
                map_id,
                spell_id,
                counter,
            ),
            Some(revision),
        ))
    }
}
