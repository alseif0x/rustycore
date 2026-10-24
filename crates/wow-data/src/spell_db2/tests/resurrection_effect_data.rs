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

#[test]
#[ignore = "requires RUSTYCORE_RESURRECTION_DATA_DIR pointing at installed Data"]
fn ghost_wisp_spell_data() {
    const SPELL_IDS: [u32; 2] = [8326, 20584];
    let spell_ids = BTreeSet::from(SPELL_IDS);
    let data_dir = std::env::var("RUSTYCORE_RESURRECTION_DATA_DIR")
        .expect("set RUSTYCORE_RESURRECTION_DATA_DIR to the installed Data directory");
    let locale = "enUS";

    let effects = SpellEffectDb2Store::load(&data_dir, locale).expect("load SpellEffect.db2");
    let effect_table_hash = effects
        .table_hash_like_cpp()
        .expect("SpellEffect.db2 store has its WDC4 table hash");
    let mut effect_rows: Vec<_> = effects
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    effect_rows.sort_unstable_by_key(|row| {
        (row.spell_id, row.difficulty_id, row.effect_index, row.id)
    });
    println!("# tableHash\tSpellEffect.db2\t0x{effect_table_hash:08X}");
    println!(
        "SpellEffect\tRecordID\tSpellID\tDifficulty\tEffectIndex\tEffect\tAuraType\tBasePoints\tDieSides\tMisc1\tMisc2\tTarget1\tTarget2\tTriggerSpell\tBonusCoefficient\tBonusCoefficientFromAP\tRealPointsPerLevel\tPointsPerResource\tCoefficient\tVariance\tResourceCoefficient\tPvPMultiplier\tGroupSizeBasePointsCoefficient"
    );
    for row in &effect_rows {
        println!(
            "SpellEffect\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
            row.id,
            row.spell_id,
            row.difficulty_id,
            row.effect_index,
            row.effect,
            row.effect_aura,
            row.effect_base_points,
            row.effect_die_sides,
            row.effect_misc_value[0],
            row.effect_misc_value[1],
            row.implicit_target[0],
            row.implicit_target[1],
            row.effect_trigger_spell,
            row.effect_bonus_coefficient,
            row.bonus_coefficient_from_ap,
            row.effect_real_points_per_level,
            row.effect_points_per_resource,
            row.coefficient,
            row.variance,
            row.resource_coefficient,
            row.pvp_multiplier,
            row.group_size_base_points_coefficient,
        );
    }
    for spell_id in SPELL_IDS {
        println!(
            "# SpellEffectCount\tspell={spell_id}\trows={}",
            effect_rows
                .iter()
                .filter(|row| row.spell_id == spell_id)
                .count()
        );
    }

    let misc = SpellMiscStore::load(&data_dir, locale).expect("load SpellMisc.db2");
    let misc_table_hash = misc
        .table_hash_like_cpp()
        .expect("SpellMisc.db2 store has its WDC4 table hash");
    let mut misc_rows: Vec<_> = misc
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    misc_rows.sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.id));
    println!("# tableHash\tSpellMisc.db2\t0x{misc_table_hash:08X}");
    println!("# SpellMiscSelected\tselectionDifficulty=0\tSpellID\tRecordID");
    for spell_id in SPELL_IDS {
        let selected = misc
            .entry_for_spell_difficulty_like_cpp(spell_id, 0)
            .map(|row| row.id);
        println!("# SpellMiscSelected\t0\t{spell_id}\t{selected:?}");
    }
    println!(
        "SpellMisc\tRecordID\tSpellID\tDifficulty\tSelectedForDifficulty0\tAttributes[0..14]\tDurationIndex\tCastingTimeIndex\tRangeIndex\tSchoolMask\tSpeed\tLaunchDelay\tMinDuration"
    );
    for row in &misc_rows {
        let attributes = row
            .attributes
            .iter()
            .map(|value| format!("0x{:08X}", *value as u32))
            .collect::<Vec<_>>()
            .join(",");
        let selected = misc
            .entry_for_spell_difficulty_like_cpp(row.spell_id, 0)
            .is_some_and(|selected| selected.id == row.id);
        println!(
            "SpellMisc\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}",
            row.id,
            row.spell_id,
            row.difficulty_id,
            selected,
            attributes,
            row.duration_index,
            row.casting_time_index,
            row.range_index,
            row.school_mask,
            row.speed,
            row.launch_delay,
            row.min_duration,
        );
    }

    let durations = SpellDurationStore::load(&data_dir, locale).expect("load SpellDuration.db2");
    let duration_table_hash = durations
        .table_hash_like_cpp()
        .expect("SpellDuration.db2 store has its WDC4 table hash");
    println!("# tableHash\tSpellDuration.db2\t0x{duration_table_hash:08X}");
    println!(
        "SpellDuration\tSpellID\tSpellMiscRecordID\tDifficulty\tDurationIndex\tRecordID\tBaseDuration\tDurationPerLevel\tMaxDuration"
    );
    for row in &misc_rows {
        match durations.get(u32::from(row.duration_index)) {
            Some(duration) => println!(
                "SpellDuration\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                row.spell_id,
                row.id,
                row.difficulty_id,
                row.duration_index,
                duration.id,
                duration.duration,
                duration.duration_per_level,
                duration.max_duration,
            ),
            None => println!(
                "# SpellDurationMissing\tSpellID={}\tSpellMiscRecordID={}\tDifficulty={}\tDurationIndex={}",
                row.spell_id, row.id, row.difficulty_id, row.duration_index,
            ),
        }
    }

    let visuals = SpellXSpellVisualStore::load(&data_dir, locale)
        .expect("load SpellXSpellVisual.db2");
    let visual_table_hash = visuals
        .table_hash_like_cpp()
        .expect("SpellXSpellVisual.db2 store has its WDC4 table hash");
    let mut visual_rows: Vec<_> = visuals
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    visual_rows.sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.id));
    println!("# tableHash\tSpellXSpellVisual.db2\t0x{visual_table_hash:08X}");
    println!(
        "SpellXSpellVisual\tRecordID\tSpellID\tDifficulty\tSpellVisualID\tProbability\tFlags\tPriority\tSpellIconFileID\tActiveIconFileID\tViewerUnitConditionID\tViewerPlayerConditionID\tCasterUnitConditionID\tCasterPlayerConditionID"
    );
    for row in &visual_rows {
        println!(
            "SpellXSpellVisual\t{}\t{}\t{}\t{}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.id,
            row.spell_id,
            row.difficulty_id,
            row.spell_visual_id,
            row.probability,
            row.flags,
            row.priority,
            row.spell_icon_file_id,
            row.active_icon_file_id,
            row.viewer_unit_condition_id,
            row.viewer_player_condition_id,
            row.caster_unit_condition_id,
            row.caster_player_condition_id,
        );
    }

    let spell_visuals = SpellVisualStore::load(&data_dir, locale).expect("load SpellVisual.db2");
    let spell_visual_table_hash = spell_visuals
        .table_hash_like_cpp()
        .expect("SpellVisual.db2 store has its WDC4 table hash");
    let related_visual_ids: BTreeSet<_> = visual_rows
        .iter()
        .map(|row| row.spell_visual_id)
        .collect();
    println!("# tableHash\tSpellVisual.db2\t0x{spell_visual_table_hash:08X}");
    println!(
        "SpellVisual\tRecordID\tHostileSpellVisualID\tCasterSpellVisualID\tFlags\tHasMissile\tSpellVisualMissileSetID\tLowViolenceSpellVisualID"
    );
    for visual_id in related_visual_ids {
        match spell_visuals.get(visual_id) {
            Some(row) => println!(
                "SpellVisual\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                row.id,
                row.hostile_spell_visual_id,
                row.caster_spell_visual_id,
                row.flags,
                row.has_missile,
                row.spell_visual_missile_set_id,
                row.low_violence_spell_visual_id,
            ),
            None => println!("# SpellVisualMissing\tRecordID={visual_id}"),
        }
    }
    for spell_id in SPELL_IDS {
        println!(
            "# SpellXSpellVisualCount\tspell={spell_id}\trows={}",
            visual_rows
                .iter()
                .filter(|row| row.spell_id == spell_id)
                .count()
        );
    }
}
