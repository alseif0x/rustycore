//! Effective ID presence for caster PlayerCondition and UnitCondition lookups.
//!
//! This catalog records whether a DB2/hotfix row exists. It does not evaluate
//! either condition's payload.

use anyhow::{Context, Result};

use crate::{Db2HotfixRemovalStoreLikeCpp, Db2IdStore};

/// Complete file stores kept private to startup until SQL additions and final
/// hotfix removals have been applied.
pub struct ConditionPresenceCatalogFiles {
    player_conditions: Db2IdStore,
    unit_conditions: Db2IdStore,
}

impl ConditionPresenceCatalogFiles {
    /// Load both complete WDC4 ID sets before querying their SQL overlays.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let player_conditions = Db2IdStore::load_complete(data_dir, locale, "PlayerCondition.db2")
            .context("failed to load complete PlayerCondition.db2 ID set")?;
        let unit_conditions = Db2IdStore::load_complete(data_dir, locale, "UnitCondition.db2")
            .context("failed to load complete UnitCondition.db2 ID set")?;
        Ok(Self {
            player_conditions,
            unit_conditions,
        })
    }

    /// Apply both SQL ID additions and then the final table-scoped removals.
    pub fn apply_hotfix_ids(
        mut self,
        player_condition_ids: impl IntoIterator<Item = u32>,
        unit_condition_ids: impl IntoIterator<Item = u32>,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<ConditionPresenceCatalog> {
        self.player_conditions
            .apply_effective_overlay(player_condition_ids, removals)
            .context("failed to apply PlayerCondition hotfix IDs")?;
        self.unit_conditions
            .apply_effective_overlay(unit_condition_ids, removals)
            .context("failed to apply UnitCondition hotfix IDs")?;
        Ok(ConditionPresenceCatalog {
            player_conditions: self.player_conditions,
            unit_conditions: self.unit_conditions,
        })
    }
}

/// Immutable effective presence for the two condition row types used by
/// `SpellInfo::GetSpellXSpellVisualId`.
pub struct ConditionPresenceCatalog {
    player_conditions: Db2IdStore,
    unit_conditions: Db2IdStore,
}

impl ConditionPresenceCatalog {
    /// Construct a known effective catalog from complete ID sets supplied by
    /// the caller. This does not load files or SQL; callers own the proof that
    /// both sets include every addition and final removal.
    pub fn from_effective_ids(
        player_condition_ids: impl IntoIterator<Item = u32>,
        unit_condition_ids: impl IntoIterator<Item = u32>,
    ) -> Self {
        Self {
            player_conditions: Db2IdStore::from_ids("PlayerCondition.db2", player_condition_ids),
            unit_conditions: Db2IdStore::from_ids("UnitCondition.db2", unit_condition_ids),
        }
    }

    pub fn contains_player_condition(&self, id: u32) -> bool {
        self.player_conditions.contains(id)
    }

    pub fn contains_unit_condition(&self, id: u32) -> bool {
        self.unit_conditions.contains(id)
    }
}

#[cfg(test)]
#[path = "condition_presence/tests.rs"]
mod tests;
