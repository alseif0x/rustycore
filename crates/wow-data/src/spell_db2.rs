//! Typed Spell* DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

mod state_1;
mod state_2;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;

#[cfg(test)]
#[path = "spell_db2/tests/mod.rs"]
mod tests;

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
            table_hash_like_cpp: Option<u32>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                    table_hash_like_cpp: None,
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            /// Iterate every effective row currently held by this DB2 store.
            ///
            /// C++ DB2 storages are iterable regardless of the concrete
            /// `Spell*Entry` type. Keeping that capability on the common Rust
            /// store wrapper avoids adding one-off iterator APIs whenever a
            /// loader needs to compose several Spell DB2 sources.
            pub fn entries_like_cpp(&self) -> impl Iterator<Item = &$entry> {
                self.entries.values()
            }
        }

        impl Db2StoreTableHashLikeCpp for $store {
            fn table_hash_like_cpp(&self) -> Option<u32> {
                self.table_hash_like_cpp
            }
        }

        impl $store {
            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(SpellAuraOptionsStore, SpellAuraOptionsEntry);
db2_store!(SpellAuraRestrictionsStore, SpellAuraRestrictionsEntry);
db2_store!(SpellCastTimesStore, SpellCastTimesEntry);
db2_store!(SpellCastingRequirementsStore, SpellCastingRequirementsEntry);
db2_store!(SpellCategoriesStore, SpellCategoriesEntry);
db2_store!(SpellCategoryStore, SpellCategoryEntry);
db2_store!(SpellClassOptionsStore, SpellClassOptionsEntry);
db2_store!(SpellCooldownsStore, SpellCooldownsEntry);
db2_store!(SpellDurationStore, SpellDurationEntry);
db2_store!(SpellEffectDb2Store, SpellEffectDb2Entry);
db2_store!(SpellEquippedItemsStore, SpellEquippedItemsEntry);
db2_store!(SpellFocusObjectStore, SpellFocusObjectEntry);
db2_store!(SpellInterruptsStore, SpellInterruptsEntry);
db2_store!(
    SpellItemEnchantmentConditionStore,
    SpellItemEnchantmentConditionEntry
);
db2_store!(SpellKeyboundOverrideStore, SpellKeyboundOverrideEntry);
db2_store!(SpellLabelStore, SpellLabelEntry);
db2_store!(SpellLearnSpellStore, SpellLearnSpellEntry);
db2_store!(SpellLevelsStore, SpellLevelsEntry);
db2_store!(SpellMiscStore, SpellMiscEntry);
db2_store!(SpellNameStore, SpellNameEntry);
db2_store!(SpellPowerStore, SpellPowerEntry);
db2_store!(SpellPowerDifficultyStore, SpellPowerDifficultyEntry);
db2_store!(SpellProcsPerMinuteStore, SpellProcsPerMinuteEntry);
db2_store!(SpellProcsPerMinuteModStore, SpellProcsPerMinuteModEntry);
db2_store!(SpellRadiusStore, SpellRadiusEntry);
db2_store!(SpellRangeStore, SpellRangeEntry);
db2_store!(SpellReagentsStore, SpellReagentsEntry);
db2_store!(SpellReagentsCurrencyStore, SpellReagentsCurrencyEntry);
db2_store!(SpellScalingStore, SpellScalingEntry);
db2_store!(SpellShapeshiftStore, SpellShapeshiftEntry);
db2_store!(SpellShapeshiftFormStore, SpellShapeshiftFormEntry);
db2_store!(SpellTargetRestrictionsStore, SpellTargetRestrictionsEntry);
db2_store!(SpellTotemsStore, SpellTotemsEntry);
db2_store!(SpellVisualStore, SpellVisualEntry);
db2_store!(SpellVisualEffectNameStore, SpellVisualEffectNameEntry);
db2_store!(SpellVisualKitStore, SpellVisualKitEntry);
db2_store!(SpellVisualMissileStore, SpellVisualMissileEntry);
db2_store!(SpellXSpellVisualStore, SpellXSpellVisualEntry);

/// The two mechanical halves every effective Spell DB2 loader shares: an SQL
/// overlay row replaces the file row with the same record ID, and the final
/// `hotfix_data` pass drops the record IDs C++ tombstoned.
///
/// Only the SQL text and the row-to-entry mapping differ per table, so those
/// stay explicit in each `load_effective_like_cpp`.
macro_rules! db2_effective_helpers {
    ($store:ident, $entry:ty, $file:literal) => {
        impl $store {
            fn overlay_effective_row_like_cpp(&mut self, entry: $entry) {
                self.entries.insert(entry.id, entry);
            }

            fn apply_hotfix_removals_with_table_hash_like_cpp(
                &mut self,
                table_hash: u32,
                removals: &crate::Db2HotfixRemovalStoreLikeCpp,
            ) {
                self.entries.retain(|record_id, _| {
                    !removals.contains_like_cpp(table_hash, *record_id as i32)
                });
            }

            fn apply_final_hotfix_removals_like_cpp(
                &mut self,
                removals: &crate::Db2HotfixRemovalStoreLikeCpp,
            ) -> Result<()> {
                let table_hash = self
                    .table_hash_like_cpp()
                    .context(concat!($file, " store is missing its WDC4 table hash"))?;
                self.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
                Ok(())
            }
        }
    };
}

db2_effective_helpers!(
    SpellCastTimesStore,
    SpellCastTimesEntry,
    "SpellCastTimes.db2"
);
db2_effective_helpers!(SpellCategoryStore, SpellCategoryEntry, "SpellCategory.db2");
db2_effective_helpers!(SpellDurationStore, SpellDurationEntry, "SpellDuration.db2");
db2_effective_helpers!(
    SpellInterruptsStore,
    SpellInterruptsEntry,
    "SpellInterrupts.db2"
);
db2_effective_helpers!(SpellRadiusStore, SpellRadiusEntry, "SpellRadius.db2");
db2_effective_helpers!(
    SpellShapeshiftStore,
    SpellShapeshiftEntry,
    "SpellShapeshift.db2"
);

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn set_table_hash_like_cpp(&mut self, table_hash: u32) {
                self.table_hash_like_cpp = Some(table_hash);
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(SpellAuraOptionsStore, SpellAuraOptionsEntry);
impl_from_entries!(SpellAuraRestrictionsStore, SpellAuraRestrictionsEntry);
impl_from_entries!(SpellCastTimesStore, SpellCastTimesEntry);
impl_from_entries!(SpellCastingRequirementsStore, SpellCastingRequirementsEntry);
impl_from_entries!(SpellCategoriesStore, SpellCategoriesEntry);
impl_from_entries!(SpellCategoryStore, SpellCategoryEntry);
impl_from_entries!(SpellClassOptionsStore, SpellClassOptionsEntry);
impl_from_entries!(SpellCooldownsStore, SpellCooldownsEntry);
impl_from_entries!(SpellDurationStore, SpellDurationEntry);
impl_from_entries!(SpellEffectDb2Store, SpellEffectDb2Entry);
impl_from_entries!(SpellEquippedItemsStore, SpellEquippedItemsEntry);
impl_from_entries!(SpellFocusObjectStore, SpellFocusObjectEntry);
impl_from_entries!(SpellInterruptsStore, SpellInterruptsEntry);
impl_from_entries!(
    SpellItemEnchantmentConditionStore,
    SpellItemEnchantmentConditionEntry
);
impl_from_entries!(SpellKeyboundOverrideStore, SpellKeyboundOverrideEntry);
impl_from_entries!(SpellLabelStore, SpellLabelEntry);
impl_from_entries!(SpellLearnSpellStore, SpellLearnSpellEntry);
impl_from_entries!(SpellLevelsStore, SpellLevelsEntry);
impl_from_entries!(SpellMiscStore, SpellMiscEntry);
impl_from_entries!(SpellNameStore, SpellNameEntry);
impl_from_entries!(SpellPowerStore, SpellPowerEntry);
impl_from_entries!(SpellPowerDifficultyStore, SpellPowerDifficultyEntry);
impl_from_entries!(SpellProcsPerMinuteStore, SpellProcsPerMinuteEntry);
impl_from_entries!(SpellProcsPerMinuteModStore, SpellProcsPerMinuteModEntry);
impl_from_entries!(SpellRadiusStore, SpellRadiusEntry);
impl_from_entries!(SpellRangeStore, SpellRangeEntry);
impl_from_entries!(SpellReagentsStore, SpellReagentsEntry);
impl_from_entries!(SpellReagentsCurrencyStore, SpellReagentsCurrencyEntry);
impl_from_entries!(SpellScalingStore, SpellScalingEntry);
impl_from_entries!(SpellShapeshiftStore, SpellShapeshiftEntry);
impl_from_entries!(SpellShapeshiftFormStore, SpellShapeshiftFormEntry);
impl_from_entries!(SpellTargetRestrictionsStore, SpellTargetRestrictionsEntry);
impl_from_entries!(SpellTotemsStore, SpellTotemsEntry);
impl_from_entries!(SpellVisualStore, SpellVisualEntry);
impl_from_entries!(SpellVisualEffectNameStore, SpellVisualEffectNameEntry);
impl_from_entries!(SpellVisualKitStore, SpellVisualKitEntry);
impl_from_entries!(SpellVisualMissileStore, SpellVisualMissileEntry);
impl_from_entries!(SpellXSpellVisualStore, SpellXSpellVisualEntry);
