// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal world spawn `guid -> entry` stores for C++ `ObjectMgr` checks.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct WorldSpawnIdStore {
    name: &'static str,
    entries_by_guid: HashMap<u32, u32>,
}

impl WorldSpawnIdStore {
    pub fn from_entries(name: &'static str, entries: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self {
            name,
            entries_by_guid: entries.into_iter().collect(),
        }
    }

    pub fn entry_for_guid(&self, guid: u32) -> Option<u32> {
        self.entries_by_guid.get(&guid).copied()
    }

    pub fn len(&self) -> usize {
        self.entries_by_guid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries_by_guid.is_empty()
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_spawn_id_store_indexes_spawn_guid_to_entry_like_object_mgr() {
        let store = WorldSpawnIdStore::from_entries("creature", [(10, 600), (11, 601)]);

        assert_eq!(store.name(), "creature");
        assert_eq!(store.entry_for_guid(10), Some(600));
        assert_eq!(store.entry_for_guid(12), None);
        assert_eq!(store.len(), 2);
    }
}
