//! Opt-in diagnostic for raw resurrection-effect rows in installed DB2 data.
//!
//! This reads `SpellEffect.db2` and `SpellMisc.db2` through production readers. It does not
//! apply server-side SQL replacements or hotfix overlays.

use super::*;
use crate::spell_db2::Db2StoreTableHashLikeCpp;
use std::collections::BTreeSet;

#[test]
#[ignore = "requires RUSTYCORE_RESURRECTION_DATA_DIR pointing at installed Data"]
fn resurrection_effect_data() {
    let data_dir = std::env::var("RUSTYCORE_RESURRECTION_DATA_DIR")
        .expect("set RUSTYCORE_RESURRECTION_DATA_DIR to the installed Data directory");
    let path = Path::new(&data_dir)
        .join("dbc")
        .join("enUS")
        .join("SpellEffect.db2");
    assert!(
        path.is_file(),
        "SpellEffect.db2 not found at {}",
        path.display()
    );

    let store = SpellEffectDb2Store::load(&data_dir, "enUS").expect("load SpellEffect.db2");
    let table_hash = store
        .table_hash_like_cpp()
        .expect("SpellEffect.db2 store has its WDC4 table hash");
    let mut rows: Vec<_> = store
        .entries_like_cpp()
        .filter(|row| matches!(row.effect, 18 | 113 | 172))
        .collect();
    rows.sort_unstable_by_key(|row| (row.spell_id, row.effect_index, row.difficulty_id, row.id));

    assert!(
        !rows.is_empty(),
        "SpellEffect.db2 has no rows for resurrection offer effects 18, 113, or 172"
    );

    println!(
        "RecordID\tspellID\teffectIndex\tdifficulty\teffect\tbasepoints\tdieSides\trealpointsperlevel\tpointsperresource\tcoefficient\tvariance\tresourcecoefficient\tmiscvalue\ttrigger"
    );
    for row in &rows {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{},{}\t{}",
            row.id,
            row.spell_id,
            row.effect_index,
            row.difficulty_id,
            row.effect,
            row.effect_base_points,
            row.effect_die_sides,
            row.effect_real_points_per_level,
            row.effect_points_per_resource,
            row.coefficient,
            row.variance,
            row.resource_coefficient,
            row.effect_misc_value[0],
            row.effect_misc_value[1],
            row.effect_trigger_spell,
        );
    }

    println!("# tableHash\t0x{table_hash:08X}");
    for effect in [18, 113, 172] {
        let matching: Vec<_> = rows.iter().filter(|row| row.effect == effect).collect();
        let nonzero = |field: fn(&SpellEffectDb2Entry) -> bool| {
            matching.iter().filter(|row| field(row)).count()
        };
        println!(
            "# summary\teffect={effect}\trows={}\tnonzero_basepoints={}\tnonzero_dieSides={}\tnonzero_realpointsperlevel={}\tnonzero_pointsperresource={}\tnonzero_coefficient={}\tnonzero_variance={}\tnonzero_resourcecoefficient={}\tnonzero_miscvalue={}\tnonzero_trigger={}",
            matching.len(),
            nonzero(|row| row.effect_base_points != 0),
            nonzero(|row| row.effect_die_sides != 0),
            nonzero(|row| row.effect_real_points_per_level != 0.0),
            nonzero(|row| row.effect_points_per_resource != 0.0),
            nonzero(|row| row.coefficient != 0.0),
            nonzero(|row| row.variance != 0.0),
            nonzero(|row| row.resource_coefficient != 0.0),
            nonzero(|row| row.effect_misc_value.iter().any(|value| *value != 0)),
            nonzero(|row| row.effect_trigger_spell != 0),
        );
    }

    let misc_store = SpellMiscStore::load(&data_dir, "enUS").expect("load SpellMisc.db2");
    let misc_table_hash = misc_store
        .table_hash_like_cpp()
        .expect("SpellMisc.db2 store has its WDC4 table hash");
    let spell_ids: BTreeSet<_> = rows.iter().map(|row| row.spell_id).collect();
    let mut misc_rows: Vec<_> = misc_store
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    misc_rows.sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.id));

    println!("# spellMiscTableHash\t0x{misc_table_hash:08X}");
    println!("# spellMiscRows\t{}", misc_rows.len());
    println!("# spellMisc\tRecordID\tSpellID\tDifficulty\tattributes[3]\tIGNORE_CASTER_MODIFIERS");
    for row in misc_rows {
        let attributes_3 = row.attributes[3] as u32;
        let ignore_caster_modifiers = attributes_3 & 0x2000_0000 != 0;
        println!(
            "# spellMisc\t{}\t{}\t{}\t0x{attributes_3:08X}\t{ignore_caster_modifiers}",
            row.id, row.spell_id, row.difficulty_id,
        );
    }
}
