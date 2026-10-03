use super::*;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};
use wow_persistence::forever::spells::server::*;
mod admission;
mod client;
mod fixtures;
mod server;
use fixtures::{client_rows, server_effect, server_spell};

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
fn build(rows: ServerSpellRows) -> SpellDefinitionSeeds {
    SpellLoadPlan::build(catalog(SpellRecords::default()))
        .unwrap()
        .with_server_spells(rows)
        .unwrap()
}
fn named(id: u32) -> SpellNameRecord {
    SpellNameRecord {
        id,
        name: SpellText::default(),
    }
}
fn difficulty(id: i16, fallback: i16) -> DifficultyRecord {
    DifficultyRecord {
        id: id as i32 as u32,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: fallback,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}
