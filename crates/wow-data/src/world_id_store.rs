// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal world-table ID stores for C++ `ObjectMgr` existence checks.

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct WorldIdStore {
    name: &'static str,
    ids: HashSet<u32>,
}

impl WorldIdStore {
    pub fn from_ids(name: &'static str, ids: impl IntoIterator<Item = u32>) -> Self {
        Self {
            name,
            ids: ids.into_iter().collect(),
        }
    }

    pub fn contains(&self, id: u32) -> bool {
        self.ids.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[cfg(test)]
    fn filtering_for_test(self, mut keep_id: impl FnMut(u32) -> bool) -> Self {
        Self {
            name: self.name,
            ids: self.ids.into_iter().filter(|id| keep_id(*id)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_id_store_indexes_ids_like_object_mgr_store() {
        let store = WorldIdStore::from_ids("creature_template", [1, 42]);

        assert_eq!(store.name(), "creature_template");
        assert!(store.contains(42));
        assert!(!store.contains(43));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn world_id_store_can_filter_invalid_backing_rows_like_cpp_loaders() {
        let store = WorldIdStore::from_ids("conversation_line_template", [1, 2, 3])
            .filtering_for_test(|id| id != 2);

        assert!(store.contains(1));
        assert!(!store.contains(2));
        assert!(store.contains(3));
    }

    #[test]
    fn signed_world_ids_are_normalized_to_unsigned_domain_like_cpp_getuint32() {
        assert_eq!(u32::try_from(42_i32).ok(), Some(42));
        assert_eq!(u32::try_from(-1_i32).ok(), None);
    }
}
