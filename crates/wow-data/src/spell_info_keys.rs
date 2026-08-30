//! Exact C++ `SpellMgr::mSpellInfoMap` key composition.
//!
//! RustyCore's [`crate::SpellStore`] intentionally hydrates only the
//! `SpellInfo` fields that have been ported so far.  That partial payload store
//! cannot also be used as proof that C++ would have constructed a
//! `(SpellID, DifficultyID)` entry: `SpellMgr::LoadSpellInfoStore` creates keys
//! from twenty independent DB2 stores before it hydrates the payload.
//!
//! This module keeps those two truths separate.  It composes only the exact
//! effective keys, including SQL replacement rows and `hotfix_data`
//! removals, without manufacturing empty Rust `SpellInfo` values.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};

use crate::Db2HotfixRemovalStoreLikeCpp;
use crate::spell_db2::{
    Db2StoreTableHashLikeCpp, SpellAuraOptionsStore, SpellAuraRestrictionsStore,
    SpellCastingRequirementsStore, SpellCategoriesStore, SpellClassOptionsStore,
    SpellCooldownsStore, SpellEffectDb2Store, SpellEquippedItemsStore, SpellInterruptsStore,
    SpellLabelStore, SpellLevelsEntry, SpellLevelsStore, SpellMiscStore, SpellNameStore,
    SpellPowerDifficultyStore, SpellPowerStore, SpellReagentsCurrencyStore, SpellReagentsStore,
    SpellScalingStore, SpellShapeshiftStore, SpellTargetRestrictionsStore, SpellTotemsStore,
    SpellXSpellVisualStore,
};

/// The exact DB2 contributors iterated by C++
/// `SpellMgr::LoadSpellInfoStore`, excluding the `SpellPowerDifficulty` join.
#[cfg(test)]
pub(crate) const SPELL_INFO_CONTRIBUTOR_TABLES_LIKE_CPP: [&str; 20] = [
    "SpellEffect.db2",
    "SpellAuraOptions.db2",
    "SpellAuraRestrictions.db2",
    "SpellCastingRequirements.db2",
    "SpellCategories.db2",
    "SpellClassOptions.db2",
    "SpellCooldowns.db2",
    "SpellEquippedItems.db2",
    "SpellInterrupts.db2",
    "SpellLabel.db2",
    "SpellLevels.db2",
    "SpellMisc.db2",
    "SpellPower.db2",
    "SpellReagents.db2",
    "SpellReagentsCurrency.db2",
    "SpellScaling.db2",
    "SpellShapeshift.db2",
    "SpellTargetRestrictions.db2",
    "SpellTotems.db2",
    "SpellXSpellVisual.db2",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellInfoKeyContributorLikeCpp {
    SpellEffect,
    SpellAuraOptions,
    SpellAuraRestrictions,
    SpellCastingRequirements,
    SpellCategories,
    SpellClassOptions,
    SpellCooldowns,
    SpellEquippedItems,
    SpellInterrupts,
    SpellLabel,
    SpellLevels,
    SpellMisc,
    SpellPower,
    SpellReagents,
    SpellReagentsCurrency,
    SpellScaling,
    SpellShapeshift,
    SpellTargetRestrictions,
    SpellTotems,
    SpellXSpellVisual,
}

pub const SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP: [SpellInfoKeyContributorLikeCpp; 20] = [
    SpellInfoKeyContributorLikeCpp::SpellEffect,
    SpellInfoKeyContributorLikeCpp::SpellAuraOptions,
    SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions,
    SpellInfoKeyContributorLikeCpp::SpellCastingRequirements,
    SpellInfoKeyContributorLikeCpp::SpellCategories,
    SpellInfoKeyContributorLikeCpp::SpellClassOptions,
    SpellInfoKeyContributorLikeCpp::SpellCooldowns,
    SpellInfoKeyContributorLikeCpp::SpellEquippedItems,
    SpellInfoKeyContributorLikeCpp::SpellInterrupts,
    SpellInfoKeyContributorLikeCpp::SpellLabel,
    SpellInfoKeyContributorLikeCpp::SpellLevels,
    SpellInfoKeyContributorLikeCpp::SpellMisc,
    SpellInfoKeyContributorLikeCpp::SpellPower,
    SpellInfoKeyContributorLikeCpp::SpellReagents,
    SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency,
    SpellInfoKeyContributorLikeCpp::SpellScaling,
    SpellInfoKeyContributorLikeCpp::SpellShapeshift,
    SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions,
    SpellInfoKeyContributorLikeCpp::SpellTotems,
    SpellInfoKeyContributorLikeCpp::SpellXSpellVisual,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellInfoKeyHotfixOverlayRowLikeCpp {
    pub record_id: u32,
    pub spell_id: u32,
    pub difficulty_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellInfoKeyHotfixOverlayBatchLikeCpp {
    pub contributor: SpellInfoKeyContributorLikeCpp,
    pub rows: Vec<SpellInfoKeyHotfixOverlayRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellInfoPowerDifficultyHotfixOverlayRowLikeCpp {
    pub power_record_id: u32,
    pub difficulty_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellInfoKeyHotfixOverlaysLikeCpp {
    pub contributor_batches: Vec<SpellInfoKeyHotfixOverlayBatchLikeCpp>,
    pub power_difficulty_rows: Vec<SpellInfoPowerDifficultyHotfixOverlayRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpellInfoSourceRowLikeCpp {
    record_id: i32,
    spell_id: u32,
    difficulty_id: u8,
}

impl SpellInfoSourceRowLikeCpp {
    const fn new(record_id: u32, spell_id: u32, difficulty_id: u8) -> Self {
        Self {
            record_id: record_id as i32,
            spell_id,
            difficulty_id,
        }
    }

    const fn signed_spell(record_id: u32, spell_id: i32, difficulty_id: u8) -> Self {
        Self::new(record_id, spell_id as u32, difficulty_id)
    }
}

#[derive(Debug, Default)]
struct EffectiveSpellInfoSourceRowsLikeCpp {
    rows_by_record_id: HashMap<i32, SpellInfoSourceRowLikeCpp>,
}

#[derive(Debug, Default)]
struct EffectivePowerDifficultyRowsLikeCpp {
    difficulty_by_power_record_id: HashMap<i32, u8>,
}

impl EffectivePowerDifficultyRowsLikeCpp {
    fn from_base_rows(rows: impl IntoIterator<Item = (i32, u8)>) -> Self {
        Self {
            difficulty_by_power_record_id: rows.into_iter().collect(),
        }
    }

    fn overlay_like_cpp(&mut self, record_id: i32, difficulty_id: u8) {
        self.difficulty_by_power_record_id
            .insert(record_id, difficulty_id);
    }

    fn apply_removals_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.difficulty_by_power_record_id
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id));
    }

    fn get(&self, power_record_id: i32) -> Option<u8> {
        self.difficulty_by_power_record_id
            .get(&power_record_id)
            .copied()
    }
}

impl EffectiveSpellInfoSourceRowsLikeCpp {
    fn from_base_rows(
        rows: impl IntoIterator<Item = SpellInfoSourceRowLikeCpp>,
    ) -> EffectiveSpellInfoSourceRowsLikeCpp {
        Self {
            rows_by_record_id: rows.into_iter().map(|row| (row.record_id, row)).collect(),
        }
    }

    fn overlay_like_cpp(&mut self, row: SpellInfoSourceRowLikeCpp) {
        self.rows_by_record_id.insert(row.record_id, row);
    }

    fn apply_removals_like_cpp(
        &mut self,
        table_hash: u32,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.rows_by_record_id
            .retain(|record_id, _| !removed_records.contains_like_cpp(table_hash, *record_id));
    }

    fn keys_like_cpp(&self) -> impl Iterator<Item = (u32, u8)> + '_ {
        self.rows_by_record_id
            .values()
            .map(|row| (row.spell_id, row.difficulty_id))
    }
}

/// Exact regular-spell keys that C++ would publish in `mSpellInfoMap`.
#[derive(Debug, Clone, Default)]
pub(crate) struct SpellInfoKeyStoreLikeCpp {
    keys: HashSet<(u32, u8)>,
    spell_ids: HashSet<u32>,
}

impl SpellInfoKeyStoreLikeCpp {
    pub(crate) fn contains_exact_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool {
        self.keys.contains(&(spell_id, difficulty_id))
    }

    pub(crate) fn contains_any_difficulty_like_cpp(&self, spell_id: u32) -> bool {
        self.spell_ids.contains(&spell_id)
    }

    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }

    pub(crate) fn exact_keys_in_order_like_cpp(&self) -> Vec<(u32, u8)> {
        let mut keys = self.keys.iter().copied().collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }

    pub(crate) fn from_candidate_keys_like_cpp(
        candidate_keys: impl IntoIterator<Item = (u32, u8)>,
        effective_spell_name_ids: &HashSet<u32>,
    ) -> Self {
        let keys: HashSet<_> = candidate_keys
            .into_iter()
            .filter(|(spell_id, _)| effective_spell_name_ids.contains(spell_id))
            .collect();
        let spell_ids = keys.iter().map(|(spell_id, _)| *spell_id).collect();
        Self { keys, spell_ids }
    }

    /// Compose the exact regular `SpellInfo` key set.
    ///
    /// C++ ordering:
    ///
    /// 1. each typed DB2 file is loaded;
    /// 2. official then custom SQL rows replace records by DB2 `RecordID`;
    /// 3. the final `hotfix_data` status removes records;
    /// 4. the twenty effective stores contribute `loadData` keys;
    /// 5. only keys with an effective `SpellName` record become `SpellInfo`.
    pub(crate) fn load_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        spell_name_store: &SpellNameStore,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
        hotfix_overlays: SpellInfoKeyHotfixOverlaysLikeCpp,
    ) -> Result<Self> {
        let effective_spell_name_ids: HashSet<u32> = spell_name_store
            .entries_like_cpp()
            .map(|entry| entry.id)
            .collect();

        let SpellInfoKeyHotfixOverlaysLikeCpp {
            contributor_batches,
            power_difficulty_rows,
        } = hotfix_overlays;
        if contributor_batches
            .iter()
            .map(|batch| batch.contributor)
            .ne(SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP)
        {
            bail!("SpellInfo key Hotfix contributor batches are missing, duplicated, or reordered");
        }

        let mut candidate_keys = HashSet::new();
        let mut contributor_batches = contributor_batches.into_iter();

        macro_rules! load_source {
            ($store:ty, $file:literal, $contributor:expr, $map:expr) => {{
                let batch = contributor_batches
                    .next()
                    .with_context(|| format!("missing {} Hotfix overlay batch", $file))?;
                debug_assert_eq!(batch.contributor, $contributor);
                let store = <$store>::load(data_dir, locale)
                    .with_context(|| format!("failed to load {}", $file))?;
                let table_hash = store
                    .table_hash_like_cpp()
                    .with_context(|| format!("{} is missing its WDC4 table hash", $file))?;
                let base_rows = store.entries_like_cpp().map($map);
                let rows = compose_effective_source_rows_like_cpp(
                    base_rows,
                    batch
                        .rows
                        .into_iter()
                        .map(spell_info_source_row_from_hotfix_like_cpp),
                    std::iter::empty(),
                    table_hash,
                    removed_records,
                );
                candidate_keys.extend(rows.keys_like_cpp());
            }};
        }

        load_source!(
            SpellEffectDb2Store,
            "SpellEffect.db2",
            SpellInfoKeyContributorLikeCpp::SpellEffect,
            |entry| SpellInfoSourceRowLikeCpp::new(
                entry.id,
                entry.spell_id,
                entry.difficulty_id as u8,
            )
        );
        load_source!(
            SpellAuraOptionsStore,
            "SpellAuraOptions.db2",
            SpellInfoKeyContributorLikeCpp::SpellAuraOptions,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );
        load_source!(
            SpellAuraRestrictionsStore,
            "SpellAuraRestrictions.db2",
            SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );
        load_source!(
            SpellCastingRequirementsStore,
            "SpellCastingRequirements.db2",
            SpellInfoKeyContributorLikeCpp::SpellCastingRequirements,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellCategoriesStore,
            "SpellCategories.db2",
            SpellInfoKeyContributorLikeCpp::SpellCategories,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );
        load_source!(
            SpellClassOptionsStore,
            "SpellClassOptions.db2",
            SpellInfoKeyContributorLikeCpp::SpellClassOptions,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellCooldownsStore,
            "SpellCooldowns.db2",
            SpellInfoKeyContributorLikeCpp::SpellCooldowns,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );
        load_source!(
            SpellEquippedItemsStore,
            "SpellEquippedItems.db2",
            SpellInfoKeyContributorLikeCpp::SpellEquippedItems,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellInterruptsStore,
            "SpellInterrupts.db2",
            SpellInfoKeyContributorLikeCpp::SpellInterrupts,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );
        load_source!(
            SpellLabelStore,
            "SpellLabel.db2",
            SpellInfoKeyContributorLikeCpp::SpellLabel,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellLevelsStore,
            "SpellLevels.db2",
            SpellInfoKeyContributorLikeCpp::SpellLevels,
            spell_levels_source_row_like_cpp
        );
        load_source!(
            SpellMiscStore,
            "SpellMisc.db2",
            SpellInfoKeyContributorLikeCpp::SpellMisc,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );

        let spell_power_batch = contributor_batches
            .next()
            .context("missing SpellPower.db2 Hotfix overlay batch")?;
        debug_assert_eq!(
            spell_power_batch.contributor,
            SpellInfoKeyContributorLikeCpp::SpellPower
        );
        let spell_power_store =
            SpellPowerStore::load(data_dir, locale).context("failed to load SpellPower.db2")?;
        let spell_power_table_hash = spell_power_store
            .table_hash_like_cpp()
            .context("SpellPower.db2 is missing its WDC4 table hash")?;
        let spell_power_rows = compose_effective_source_rows_like_cpp(
            spell_power_store
                .entries_like_cpp()
                .map(|entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, 0)),
            spell_power_batch
                .rows
                .into_iter()
                .map(spell_info_source_row_from_hotfix_like_cpp),
            std::iter::empty(),
            spell_power_table_hash,
            removed_records,
        );
        let spell_power_difficulties = load_effective_power_difficulties_from_hotfix_rows_like_cpp(
            data_dir,
            locale,
            removed_records,
            power_difficulty_rows,
        )?;
        candidate_keys.extend(power_keys_like_cpp(
            &spell_power_rows,
            &spell_power_difficulties,
        ));

        load_source!(
            SpellReagentsStore,
            "SpellReagents.db2",
            SpellInfoKeyContributorLikeCpp::SpellReagents,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellReagentsCurrencyStore,
            "SpellReagentsCurrency.db2",
            SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellScalingStore,
            "SpellScaling.db2",
            SpellInfoKeyContributorLikeCpp::SpellScaling,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellShapeshiftStore,
            "SpellShapeshift.db2",
            SpellInfoKeyContributorLikeCpp::SpellShapeshift,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellTargetRestrictionsStore,
            "SpellTargetRestrictions.db2",
            SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );
        load_source!(
            SpellTotemsStore,
            "SpellTotems.db2",
            SpellInfoKeyContributorLikeCpp::SpellTotems,
            |entry| SpellInfoSourceRowLikeCpp::signed_spell(entry.id, entry.spell_id, 0)
        );
        load_source!(
            SpellXSpellVisualStore,
            "SpellXSpellVisual.db2",
            SpellInfoKeyContributorLikeCpp::SpellXSpellVisual,
            |entry| SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id,)
        );

        debug_assert!(contributor_batches.next().is_none());
        Ok(Self::from_candidate_keys_like_cpp(
            candidate_keys,
            &effective_spell_name_ids,
        ))
    }
}

fn compose_effective_source_rows_like_cpp(
    base_rows: impl IntoIterator<Item = SpellInfoSourceRowLikeCpp>,
    official_overlay_rows: impl IntoIterator<Item = SpellInfoSourceRowLikeCpp>,
    custom_overlay_rows: impl IntoIterator<Item = SpellInfoSourceRowLikeCpp>,
    table_hash: u32,
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
) -> EffectiveSpellInfoSourceRowsLikeCpp {
    let mut effective = EffectiveSpellInfoSourceRowsLikeCpp::from_base_rows(base_rows);
    for row in official_overlay_rows {
        effective.overlay_like_cpp(row);
    }
    for row in custom_overlay_rows {
        effective.overlay_like_cpp(row);
    }
    effective.apply_removals_like_cpp(table_hash, removed_records);
    effective
}

fn load_effective_power_difficulties_from_hotfix_rows_like_cpp(
    data_dir: &str,
    locale: &str,
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
    hotfix_rows: Vec<SpellInfoPowerDifficultyHotfixOverlayRowLikeCpp>,
) -> Result<EffectivePowerDifficultyRowsLikeCpp> {
    let store = SpellPowerDifficultyStore::load(data_dir, locale)
        .context("failed to load SpellPowerDifficulty.db2")?;
    let table_hash = store
        .table_hash_like_cpp()
        .context("SpellPowerDifficulty.db2 is missing its WDC4 table hash")?;
    let mut rows = EffectivePowerDifficultyRowsLikeCpp::from_base_rows(
        store
            .entries_like_cpp()
            .map(|entry| (entry.id as i32, entry.difficulty_id)),
    );

    for row in hotfix_rows {
        rows.overlay_like_cpp(row.power_record_id as i32, row.difficulty_id);
    }

    rows.apply_removals_like_cpp(table_hash, removed_records);
    Ok(rows)
}

fn power_keys_like_cpp<'a>(
    power_rows: &'a EffectiveSpellInfoSourceRowsLikeCpp,
    power_difficulties: &'a EffectivePowerDifficultyRowsLikeCpp,
) -> impl Iterator<Item = (u32, u8)> + 'a {
    power_rows.rows_by_record_id.values().map(|row| {
        (
            row.spell_id,
            power_difficulties.get(row.record_id).unwrap_or(0),
        )
    })
}

fn spell_levels_source_row_like_cpp(entry: &SpellLevelsEntry) -> SpellInfoSourceRowLikeCpp {
    SpellInfoSourceRowLikeCpp::new(entry.id, entry.spell_id, entry.difficulty_id)
}

fn spell_info_source_row_from_hotfix_like_cpp(
    row: SpellInfoKeyHotfixOverlayRowLikeCpp,
) -> SpellInfoSourceRowLikeCpp {
    SpellInfoSourceRowLikeCpp::new(row.record_id, row.spell_id, row.difficulty_id)
}

#[cfg(test)]
mod tests {
    use super::{
        EffectivePowerDifficultyRowsLikeCpp, EffectiveSpellInfoSourceRowsLikeCpp,
        SPELL_INFO_CONTRIBUTOR_TABLES_LIKE_CPP, SpellInfoKeyStoreLikeCpp,
        SpellInfoSourceRowLikeCpp, compose_effective_source_rows_like_cpp, power_keys_like_cpp,
        spell_levels_source_row_like_cpp,
    };
    use crate::{Db2HotfixRemovalStoreLikeCpp, SpellLevelsEntry};
    use std::collections::HashSet;

    #[test]
    fn contributor_manifest_matches_cpp_load_spell_info_store() {
        assert_eq!(
            SPELL_INFO_CONTRIBUTOR_TABLES_LIKE_CPP,
            [
                "SpellEffect.db2",
                "SpellAuraOptions.db2",
                "SpellAuraRestrictions.db2",
                "SpellCastingRequirements.db2",
                "SpellCategories.db2",
                "SpellClassOptions.db2",
                "SpellCooldowns.db2",
                "SpellEquippedItems.db2",
                "SpellInterrupts.db2",
                "SpellLabel.db2",
                "SpellLevels.db2",
                "SpellMisc.db2",
                "SpellPower.db2",
                "SpellReagents.db2",
                "SpellReagentsCurrency.db2",
                "SpellScaling.db2",
                "SpellShapeshift.db2",
                "SpellTargetRestrictions.db2",
                "SpellTotems.db2",
                "SpellXSpellVisual.db2",
            ]
        );
    }

    #[test]
    fn name_only_does_not_create_spell_info_key() {
        let names = HashSet::from([100]);
        let store = SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp([], &names);

        assert!(!store.contains_exact_like_cpp(100, 0));
    }

    #[test]
    fn any_named_contributor_creates_exact_key_without_payload_claim() {
        let names = HashSet::from([100]);
        let store = SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp([(100, 0)], &names);

        assert!(store.contains_exact_like_cpp(100, 0));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn contributor_without_name_does_not_create_spell_info_key() {
        let store =
            SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp([(100, 0)], &HashSet::new());

        assert!(!store.contains_exact_like_cpp(100, 0));
    }

    #[test]
    fn difficulty_specific_contributor_does_not_create_difficulty_none() {
        let names = HashSet::from([100]);
        let store = SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp([(100, 2)], &names);

        assert!(store.contains_exact_like_cpp(100, 2));
        assert!(!store.contains_exact_like_cpp(100, 0));
        assert!(store.contains_any_difficulty_like_cpp(100));
        assert!(!store.contains_any_difficulty_like_cpp(200));
    }

    #[test]
    fn duplicate_contributors_deduplicate_exact_keys() {
        let names = HashSet::from([100]);
        let store = SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            [(100, 0), (100, 0), (100, 0)],
            &names,
        );

        assert_eq!(store.len(), 1);
    }

    #[test]
    fn official_then_custom_overlays_replace_relations_by_record_id() {
        let rows = compose_effective_source_rows_like_cpp(
            [
                SpellInfoSourceRowLikeCpp::new(7, 100, 0),
                SpellInfoSourceRowLikeCpp::new(8, 200, 0),
            ],
            [SpellInfoSourceRowLikeCpp::new(7, 101, 1)],
            [SpellInfoSourceRowLikeCpp::new(7, 102, 2)],
            0xAABB_CCDD,
            &Db2HotfixRemovalStoreLikeCpp::default(),
        );

        let keys: HashSet<_> = rows.keys_like_cpp().collect();
        assert_eq!(keys, HashSet::from([(102, 2), (200, 0)]));
    }

    #[test]
    fn sql_overlay_can_add_a_new_contributor_record() {
        let rows = compose_effective_source_rows_like_cpp(
            [],
            [SpellInfoSourceRowLikeCpp::new(7, 100, 0)],
            [],
            0xAABB_CCDD,
            &Db2HotfixRemovalStoreLikeCpp::default(),
        );

        assert_eq!(rows.keys_like_cpp().collect::<Vec<_>>(), vec![(100, 0)]);
    }

    #[test]
    fn final_record_removed_erases_effective_contributor() {
        let removals =
            Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(0xAABB_CCDD, 7, 2)]);
        let rows = compose_effective_source_rows_like_cpp(
            [
                SpellInfoSourceRowLikeCpp::new(7, 100, 0),
                SpellInfoSourceRowLikeCpp::new(8, 200, 0),
            ],
            [],
            [],
            0xAABB_CCDD,
            &removals,
        );

        assert_eq!(rows.keys_like_cpp().collect::<Vec<_>>(), vec![(200, 0)]);
    }

    #[test]
    fn spell_power_uses_effective_difficulty_row_or_none() {
        let powers = EffectiveSpellInfoSourceRowsLikeCpp::from_base_rows([
            SpellInfoSourceRowLikeCpp::new(7, 100, 0),
            SpellInfoSourceRowLikeCpp::new(8, 200, 0),
        ]);
        let difficulties = EffectivePowerDifficultyRowsLikeCpp::from_base_rows([(7, 3)]);

        assert_eq!(
            power_keys_like_cpp(&powers, &difficulties).collect::<HashSet<_>>(),
            HashSet::from([(100, 3), (200, 0)])
        );
    }

    #[test]
    fn spell_power_difficulty_overlay_and_removal_use_power_record_id() {
        let mut difficulties =
            EffectivePowerDifficultyRowsLikeCpp::from_base_rows([(7, 1), (8, 2)]);
        difficulties.overlay_like_cpp(7, 3);
        let removals =
            Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(0xAABB_CCDD, 8, 2)]);
        difficulties.apply_removals_like_cpp(0xAABB_CCDD, &removals);

        assert_eq!(difficulties.get(7), Some(3));
        assert_eq!(difficulties.get(8), None);
    }

    #[test]
    fn spell_levels_real_entry_normalizes_as_a_cpp_contributor() {
        let row = spell_levels_source_row_like_cpp(&SpellLevelsEntry {
            id: 7,
            difficulty_id: 0,
            base_level: 1,
            max_level: 80,
            spell_level: 10,
            max_passive_aura_level: 0,
            spell_id: 100,
        });

        assert_eq!(row, SpellInfoSourceRowLikeCpp::new(7, 100, 0));
    }

    #[test]
    fn empty_spell_name_text_is_irrelevant_to_presence() {
        let effective_name_ids = HashSet::from([100]);
        let store =
            SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp([(100, 0)], &effective_name_ids);

        assert!(store.contains_exact_like_cpp(100, 0));
    }
}
