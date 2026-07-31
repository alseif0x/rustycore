//! Effective DB2 record removals from C++ `DB2Manager::LoadHotfixData`.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use wow_database::{HotfixDatabase, HotfixStatements, SqlResult};

/// Records whose final `hotfix_data` status is `RecordRemoved`.
///
/// `hotfix_data` is ordered by push id in C++. A later status for the same
/// `(TableHash, RecordID)` replaces the earlier removal decision.
#[derive(Debug, Clone, Default)]
pub struct Db2HotfixRemovalStoreLikeCpp {
    removed_records: HashSet<(u32, i32)>,
}

impl Db2HotfixRemovalStoreLikeCpp {
    pub async fn load_like_cpp(hotfix_db: &HotfixDatabase) -> Result<Self> {
        let statement = hotfix_db.prepare(HotfixStatements::SEL_HOTFIX_DATA);
        let mut result = hotfix_db
            .query(&statement)
            .await
            .context("failed to load effective hotfix_data removals")?;
        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut status_rows = Vec::with_capacity(result.count());
        loop {
            status_rows.push((
                read_u32_like_cpp(&result, 2),
                read_i32_like_cpp(&result, 3),
                read_u8_like_cpp(&result, 4),
            ));
            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_status_rows_like_cpp(status_rows))
    }

    pub fn contains_like_cpp(&self, table_hash: u32, record_id: i32) -> bool {
        self.removed_records.contains(&(table_hash, record_id))
    }

    pub fn len(&self) -> usize {
        self.removed_records.len()
    }

    /// Stable evidence iterator for specialized effective-store projections.
    ///
    /// Callers must not infer a relation owner for a tombstone whose payload
    /// was absent; this only exposes the final `(TableHash, RecordID)` status.
    pub fn removed_records_in_order_like_cpp(&self) -> Vec<(u32, i32)> {
        let mut records = self.removed_records.iter().copied().collect::<Vec<_>>();
        records.sort_unstable();
        records
    }

    pub(crate) fn from_status_rows_like_cpp(
        status_rows_in_push_order: impl IntoIterator<Item = (u32, i32, u8)>,
    ) -> Self {
        let mut final_status_by_record = HashMap::new();
        for (table_hash, record_id, status) in status_rows_in_push_order {
            // C++ deliberately assigns, rather than ORs, this decision:
            // `deletedRecords[{ tableHash, recordId }] =
            //     status == HotfixRecord::Status::RecordRemoved`.
            // The last accepted row in the `ORDER BY Id` result therefore
            // controls the post-query `EraseRecord` pass.
            final_status_by_record.insert((table_hash, record_id), status);
        }

        Self {
            removed_records: final_status_by_record
                .into_iter()
                .filter_map(|(key, status)| (status == 2).then_some(key))
                .collect(),
        }
    }
}

fn read_u32_like_cpp(result: &SqlResult, column: usize) -> u32 {
    result
        .try_read::<u32>(column)
        .or_else(|| result.try_read::<i32>(column).map(|value| value as u32))
        .or_else(|| result.try_read::<u64>(column).map(|value| value as u32))
        .or_else(|| result.try_read::<i64>(column).map(|value| value as u32))
        .unwrap_or(0)
}

fn read_i32_like_cpp(result: &SqlResult, column: usize) -> i32 {
    result
        .try_read::<i32>(column)
        .or_else(|| result.try_read::<u32>(column).map(|value| value as i32))
        .or_else(|| result.try_read::<i64>(column).map(|value| value as i32))
        .or_else(|| result.try_read::<u64>(column).map(|value| value as i32))
        .unwrap_or(0)
}

fn read_u8_like_cpp(result: &SqlResult, column: usize) -> u8 {
    result
        .try_read::<u8>(column)
        .or_else(|| result.try_read::<u16>(column).map(|value| value as u8))
        .or_else(|| result.try_read::<u32>(column).map(|value| value as u8))
        .or_else(|| result.try_read::<i32>(column).map(|value| value as u8))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::Db2HotfixRemovalStoreLikeCpp;

    #[test]
    fn latest_status_controls_record_removal_like_cpp() {
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (0xAAAA, 7, 2),
            (0xBBBB, 8, 1),
            (0xAAAA, 7, 1),
            (0xBBBB, 8, 2),
        ]);

        assert!(!removals.contains_like_cpp(0xAAAA, 7));
        assert!(removals.contains_like_cpp(0xBBBB, 8));
        assert_eq!(removals.len(), 1);
    }

    #[test]
    fn final_removal_evidence_is_stably_ordered() {
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (0xBBBB, 8, 2),
            (0xAAAA, -1, 2),
            (0xAAAA, 7, 2),
        ]);

        assert_eq!(
            removals.removed_records_in_order_like_cpp(),
            vec![(0xAAAA, -1), (0xAAAA, 7), (0xBBBB, 8)]
        );
    }
}
