// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal world-table ID stores for C++ `ObjectMgr` existence checks.

use std::collections::HashSet;

use anyhow::{Result, bail};
use wow_database::{SqlResult, WorldDatabase, WorldStatements};

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

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        name: &'static str,
        statement: WorldStatements,
    ) -> Result<Self> {
        Self::load_filtering_like_cpp(db, name, statement, |_| true).await
    }

    pub async fn load_filtering_like_cpp(
        db: &WorldDatabase,
        name: &'static str,
        statement: WorldStatements,
        mut keep_id: impl FnMut(u32) -> bool,
    ) -> Result<Self> {
        let stmt = db.prepare(statement);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(Self::from_ids(name, []));
        }

        let mut ids = HashSet::new();
        loop {
            let Some(id) = read_world_id_like_cpp(&result, 0, name)? else {
                if !result.next_row() {
                    break;
                }
                continue;
            };
            if keep_id(id) {
                ids.insert(id);
            }
            if !result.next_row() {
                break;
            }
        }

        Ok(Self { name, ids })
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

fn read_world_id_like_cpp(
    result: &SqlResult,
    column: usize,
    store_name: &'static str,
) -> Result<Option<u32>> {
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(Some(value));
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(Some(u32::from(value)));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(Some(u32::from(value)));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return Ok(u32::try_from(value).ok());
    }

    bail!("unsupported ID column type while loading world id store `{store_name}`")
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
