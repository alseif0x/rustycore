// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadReservedPlayersNames` data store.

use std::collections::HashSet;

use anyhow::Result;
use wow_database::{CharStatements, CharacterDatabase};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReservedNameStoreLikeCpp {
    names: HashSet<String>,
    loaded_rows: usize,
}

impl ReservedNameStoreLikeCpp {
    pub fn from_names_like_cpp(names: impl IntoIterator<Item = String>) -> Self {
        let mut store = Self::default();
        for name in names {
            store.loaded_rows += 1;
            store.names.insert(normalize_reserved_name_like_cpp(&name));
        }
        store
    }

    /// C++ `ObjectMgr::LoadReservedPlayersNames`.
    pub async fn load_like_cpp(db: &CharacterDatabase) -> Result<Self> {
        let stmt = db.prepare(CharStatements::SEL_RESERVED_NAMES);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(result.read::<String>(0));
                if !result.next_row() {
                    break;
                }
            }
        }

        let store = Self::from_names_like_cpp(rows);
        Ok(store)
    }

    /// C++ `ObjectMgr::IsReservedName`.
    pub fn is_reserved_name_like_cpp(&self, name: &str) -> bool {
        self.names.contains(&normalize_reserved_name_like_cpp(name))
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn loaded_rows_like_cpp(&self) -> usize {
        self.loaded_rows
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

fn normalize_reserved_name_like_cpp(name: &str) -> String {
    // C++ converts UTF-8 to wide chars and applies `wstrToLower`. Rust stores
    // valid UTF-8 already; `to_lowercase` is the closest Unicode-aware mirror.
    name.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_names_match_case_insensitively_like_cpp() {
        let store = ReservedNameStoreLikeCpp::from_names_like_cpp([
            "Arthas".to_string(),
            "Thrall".to_string(),
        ]);

        assert!(store.is_reserved_name_like_cpp("arthas"));
        assert!(store.is_reserved_name_like_cpp("ARTHAS"));
        assert!(store.is_reserved_name_like_cpp("Thrall"));
        assert!(!store.is_reserved_name_like_cpp("Jaina"));
        assert_eq!(store.len(), 2);
        assert_eq!(store.loaded_rows_like_cpp(), 2);
    }

    #[test]
    fn reserved_names_duplicate_rows_count_like_cpp_but_store_unique() {
        let store = ReservedNameStoreLikeCpp::from_names_like_cpp([
            "Arthas".to_string(),
            "arthas".to_string(),
        ]);

        assert!(store.is_reserved_name_like_cpp("ARTHAS"));
        assert_eq!(store.len(), 1);
        assert_eq!(store.loaded_rows_like_cpp(), 2);
    }
}
