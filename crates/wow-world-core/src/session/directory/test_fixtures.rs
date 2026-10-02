// Copyright (c) 2026 alseif0x
//! Feature-gated fixture accessors for the session directory.

use super::*;

impl PlayerRegistry {
    /// Clone the only non-canonical fixture value without exposing storage.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_active_loot_rolls(
        &self,
        guid: ObjectGuid,
    ) -> Option<Vec<LootRollCommandIdentityLikeCpp>> {
        self.entries
            .get(&guid)
            .map(|entry| entry.active_loot_rolls.clone())
    }

    /// Clone one fixture entry's durable creature rail. The projection no
    /// longer carries it (#270), and a test that drains the rail is addressing
    /// the entry, not reading gameplay state.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_durable_creature_runtime_commands_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>> {
        self.entries
            .get(&guid)
            .map(|entry| Arc::clone(&entry.durable_creature_runtime_commands_like_cpp))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_update(
        &self,
        guid: ObjectGuid,
        update: impl FnOnce(&mut PlayerDirectoryPlacementLikeCpp),
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        let entry = &mut *entry;
        update(&mut entry.placement);
        true
    }

    /// Remove one fixture registration without exporting its storage record.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_remove(&self, guid: ObjectGuid) -> bool {
        self.entries.remove(&guid).is_some()
    }

    /// Count connected fixture registrations without exposing iteration.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/directory/test_fixtures.rs"]
mod tests;
