use anyhow::Result;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, SpellAuraOptionsEntry, SpellAuraOptionsStore,
    SpellClassOptionsEntry, SpellClassOptionsStore, SpellLabelEntry, SpellLabelStore,
    SpellLevelsEntry, SpellLevelsStore, SpellProcsPerMinuteEntry, SpellProcsPerMinuteStore,
};
use wow_persistence::{
    SpellAuraOptionsHotfixRow, SpellClassOptionsHotfixRow,
    SpellCoreDb2HotfixPersistencePortLikeCpp, SpellLabelHotfixRow, SpellLevelsHotfixRow,
    SpellProcsPerMinuteHotfixRow,
};

use super::loaded_rows_like_cpp;

pub(crate) async fn load_spell_aura_options_store(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellAuraOptionsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_aura_options_rows().await)?;
    SpellAuraOptionsStore::load_effective_from_hotfix_rows(
        data_dir,
        locale,
        rows.into_iter().map(spell_aura_options_entry),
        removals,
    )
}

pub(crate) async fn load_spell_class_options_store(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellClassOptionsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_class_options_rows().await)?;
    SpellClassOptionsStore::load_effective_from_hotfix_rows(
        data_dir,
        locale,
        rows.into_iter().map(spell_class_options_entry),
        removals,
    )
}

pub(crate) async fn load_spell_label_store(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellLabelStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_label_rows().await)?;
    SpellLabelStore::load_effective_from_hotfix_rows(
        data_dir,
        locale,
        rows.into_iter().map(spell_label_entry),
        removals,
    )
}

pub(crate) async fn load_spell_levels_store(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellLevelsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_levels_rows().await)?;
    SpellLevelsStore::load_effective_from_hotfix_rows(
        data_dir,
        locale,
        rows.into_iter().map(spell_levels_entry),
        removals,
    )
}

pub(crate) async fn load_spell_procs_per_minute_store(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellProcsPerMinuteStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_procs_per_minute_rows().await)?;
    SpellProcsPerMinuteStore::load_effective_from_hotfix_rows(
        data_dir,
        locale,
        rows.into_iter().map(spell_procs_per_minute_entry),
        removals,
    )
}

fn spell_aura_options_entry(row: SpellAuraOptionsHotfixRow) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        cumulative_aura: row.cumulative_aura,
        proc_category_recovery: row.proc_category_recovery,
        proc_chance: row.proc_chance,
        proc_charges: row.proc_charges,
        spell_procs_per_minute_id: row.spell_procs_per_minute_id,
        proc_type_mask: row.proc_type_mask,
        spell_id: row.spell_id,
    }
}

fn spell_class_options_entry(row: SpellClassOptionsHotfixRow) -> SpellClassOptionsEntry {
    SpellClassOptionsEntry {
        id: row.id,
        spell_id: row.spell_id,
        modal_next_spell: row.modal_next_spell,
        spell_class_set: row.spell_class_set,
        spell_class_mask: row.spell_class_mask.map(|mask| mask as u32),
    }
}

fn spell_label_entry(row: SpellLabelHotfixRow) -> SpellLabelEntry {
    SpellLabelEntry {
        id: row.id,
        label_id: row.label_id,
        spell_id: row.spell_id,
    }
}

fn spell_levels_entry(row: SpellLevelsHotfixRow) -> SpellLevelsEntry {
    SpellLevelsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        base_level: row.base_level,
        max_level: row.max_level,
        spell_level: row.spell_level,
        max_passive_aura_level: row.max_passive_aura_level,
        spell_id: row.spell_id,
    }
}

fn spell_procs_per_minute_entry(row: SpellProcsPerMinuteHotfixRow) -> SpellProcsPerMinuteEntry {
    SpellProcsPerMinuteEntry {
        id: row.id,
        base_proc_rate: row.base_proc_rate,
        flags: row.flags,
    }
}

#[cfg(test)]
#[path = "selected_scalars_tests.rs"]
mod tests;
