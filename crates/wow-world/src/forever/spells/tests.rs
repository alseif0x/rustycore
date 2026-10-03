use super::*;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};
mod constructor;
mod fixtures;
mod joins;
mod safety;
mod side_indices;
mod traversal;
use fixtures::*;

fn named(ids: &[u32]) -> SpellRecords {
    SpellRecords {
        spell_names: ids
            .iter()
            .map(|&id| SpellNameRecord {
                id,
                name: SpellText::default(),
            })
            .collect(),
        ..Default::default()
    }
}
fn catalog(rows: SpellRecords) -> Arc<SpellCatalog> {
    Arc::new(
        rows.finish(
            SpellRecords::default(),
            SpellRecords::default(),
            6,
            SpellLocaleRecords::default(),
            SpellLocaleRecords::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
        )
        .unwrap(),
    )
}
fn plan(rows: SpellRecords) -> SpellLoadPlan {
    SpellLoadPlan::build(catalog(rows)).unwrap()
}
fn difficulty(id: i16, fallback: i16) -> DifficultyRecord {
    let mut row = fixtures::difficulty(id as i32 as u32, 0);
    row.fallback_difficulty_id = fallback;
    row
}
fn effect(id: u32, spell: u32, difficulty: i16, slot: i32) -> SpellEffectRecord {
    let mut row = spell_effect(id, spell);
    row.difficulty_id = difficulty;
    row.effect_index = slot;
    row
}
fn visual(id: u32, difficulty: i16, priority: i32, condition: u32) -> SpellXSpellVisualRecord {
    let mut row = spell_x_spell_visual(id, 1);
    row.difficulty_id = difficulty;
    row.priority = priority;
    row.caster_player_condition_id = condition;
    row
}
