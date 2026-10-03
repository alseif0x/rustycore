//! Target typed-hotfix startup queries; no SQL/key data reaches session logs.
//! 02245dcd HotfixDatabase.cpp:1820-1823, DB2DatabaseLoader.cpp:28-33.

use crate::{HotfixDatabase, PreparedStatement};
use std::{collections::BTreeSet, sync::Arc};
use wow_persistence::forever::{LoadError, TactKeyOverlays, TactKeyRow};
mod appearance;

const TACT_KEY_QUERY: &str = "SELECT ID,Key1,Key2,Key3,Key4,Key5,Key6,Key7,Key8,Key9,Key10,Key11,Key12,Key13,Key14,Key15,Key16 FROM tact_key WHERE (VerifiedBuild>0)=?";

pub struct ForeverHotfixRepository(Arc<HotfixDatabase>);

impl ForeverHotfixRepository {
    pub fn new(database: Arc<HotfixDatabase>) -> Self {
        Self(database)
    }

    pub async fn load_tact_key_overlays(&self) -> Result<TactKeyOverlays, LoadError> {
        // DB2Storage loads official rows before its custom overlay. A failed
        // second query must not leave a partially published effective store.
        let official = self.load(false).await?;
        let custom = self.load(true).await?;
        Ok(TactKeyOverlays { official, custom })
    }

    async fn load(&self, custom: bool) -> Result<Vec<TactKeyRow>, LoadError> {
        let mut query = PreparedStatement::new(TACT_KEY_QUERY);
        query.set_bool(0, !custom);
        let mut result = self
            .0
            .query(&query)
            .await
            .map_err(|_| LoadError::Database)?;
        let mut rows = Vec::new();
        let mut ids = BTreeSet::new();
        if result.is_empty() {
            return Ok(rows);
        }
        loop {
            let id = result.try_read::<u32>(0).ok_or(LoadError::InvalidRow)?;
            // A duplicate inside one unordered batch has no defined winner;
            // reject it instead of inventing a target precedence rule.
            if !ids.insert(id) {
                return Err(LoadError::InvalidRow);
            }
            let mut key = [0; 16];
            for (index, byte) in key.iter_mut().enumerate() {
                *byte = result
                    .try_read::<u8>(index + 1)
                    .ok_or(LoadError::InvalidRow)?;
            }
            rows.push(TactKeyRow { id, key });
            if !result.next_row() {
                break;
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_matches_target_columns_and_verified_build_predicate_without_locale_filter() {
        assert_eq!(
            TACT_KEY_QUERY
                .split(" FROM ")
                .next()
                .unwrap()
                .split(',')
                .count(),
            17
        );
        assert!(TACT_KEY_QUERY.ends_with("FROM tact_key WHERE (VerifiedBuild>0)=?"));
        assert!(!TACT_KEY_QUERY.contains("locale"));
        assert!(!TACT_KEY_QUERY.contains("SELECT *"));
    }
}
