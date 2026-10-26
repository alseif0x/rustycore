//! Complete and effective loading for the selected scalar Spell DB2 catalogs.

use anyhow::{Context, Result};

use super::{
    Db2StoreTableHashLikeCpp, SpellAuraOptionsEntry, SpellAuraOptionsStore, SpellClassOptionsEntry,
    SpellClassOptionsStore, SpellLabelEntry, SpellLabelStore, SpellLevelsEntry, SpellLevelsStore,
    load_store_complete,
};
use crate::{Db2HotfixRemovalStoreLikeCpp, wdc4::Wdc4Reader};

pub(super) fn read_spell_aura_options_entry(
    id: u32,
    idx: usize,
    reader: &Wdc4Reader,
) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id,
        difficulty_id: reader.get_field_u8(idx, 0),
        cumulative_aura: reader.get_field_u32(idx, 1),
        proc_category_recovery: reader.get_field_i32(idx, 2),
        proc_chance: reader.get_field_u8(idx, 3),
        proc_charges: reader.get_field_i32(idx, 4),
        spell_procs_per_minute_id: reader.get_field_u16(idx, 5),
        proc_type_mask: std::array::from_fn(|i| reader.get_array_element(idx, 6, i, 32) as i32),
        spell_id: reader.get_relationship_id(idx).unwrap_or(0),
    }
}

pub(super) fn read_spell_class_options_entry(
    id: u32,
    idx: usize,
    reader: &Wdc4Reader,
) -> SpellClassOptionsEntry {
    SpellClassOptionsEntry {
        id,
        spell_id: reader.get_field_i32(idx, 0),
        modal_next_spell: reader.get_field_u32(idx, 1),
        spell_class_set: reader.get_field_u8(idx, 2),
        spell_class_mask: std::array::from_fn(|i| reader.get_array_element(idx, 3, i, 32)),
    }
}

pub(super) fn read_spell_label_entry(id: u32, idx: usize, reader: &Wdc4Reader) -> SpellLabelEntry {
    SpellLabelEntry {
        id,
        label_id: reader.get_field_u32(idx, 0),
        spell_id: reader.get_relationship_id(idx).unwrap_or(0),
    }
}

pub(super) fn read_spell_levels_entry(
    id: u32,
    idx: usize,
    reader: &Wdc4Reader,
) -> SpellLevelsEntry {
    SpellLevelsEntry {
        id,
        difficulty_id: reader.get_field_u8(idx, 0),
        base_level: reader.get_field_i16(idx, 1),
        max_level: reader.get_field_i16(idx, 2),
        spell_level: reader.get_field_i16(idx, 3),
        max_passive_aura_level: reader.get_field_u8(idx, 4),
        spell_id: reader.get_relationship_id(idx).unwrap_or(0),
    }
}

macro_rules! impl_selected_scalar_loaders {
    ($store:ty, $entry:ty, $file_name:literal, $reader:ident) => {
        impl $store {
            /// Load every declared WDC4 record, rejecting unresolved copy rows.
            pub fn load_complete(data_dir: &str, locale: &str) -> Result<Self> {
                load_store_complete(data_dir, locale, $file_name, $reader)
            }

            /// Apply ordered official then custom rows after the complete file,
            /// then apply final removals keyed by the file's table hash.
            pub fn load_effective_from_hotfix_rows(
                data_dir: &str,
                locale: &str,
                rows: impl IntoIterator<Item = $entry>,
                removals: &Db2HotfixRemovalStoreLikeCpp,
            ) -> Result<Self> {
                let mut store = Self::load_complete(data_dir, locale)?;
                for row in rows {
                    store.entries.insert(row.id, row);
                }

                let table_hash = store
                    .table_hash_like_cpp()
                    .context(concat!($file_name, " store is missing its WDC4 table hash"))?;
                store.entries.retain(|record_id, _| {
                    !removals.contains_like_cpp(table_hash, *record_id as i32)
                });
                Ok(store)
            }
        }
    };
}

impl_selected_scalar_loaders!(
    SpellAuraOptionsStore,
    SpellAuraOptionsEntry,
    "SpellAuraOptions.db2",
    read_spell_aura_options_entry
);
impl_selected_scalar_loaders!(
    SpellClassOptionsStore,
    SpellClassOptionsEntry,
    "SpellClassOptions.db2",
    read_spell_class_options_entry
);
impl_selected_scalar_loaders!(
    SpellLabelStore,
    SpellLabelEntry,
    "SpellLabel.db2",
    read_spell_label_entry
);
impl_selected_scalar_loaders!(
    SpellLevelsStore,
    SpellLevelsEntry,
    "SpellLevels.db2",
    read_spell_levels_entry
);
