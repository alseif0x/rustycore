//! Opt-in diagnostic for raw resurrection-effect rows in installed DB2 data.
//!
//! This reads `SpellEffect.db2` and `SpellMisc.db2` through production readers. It does not
//! apply server-side SQL replacements or hotfix overlays.

use super::*;
use crate::spell_db2::Db2StoreTableHashLikeCpp;
use crate::wdc4::Wdc4Reader;
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
    const GHOST_WISP_SPELL_IDS: [u32; 2] = [8326, 20584];
    let data_dir = std::env::var("RUSTYCORE_RESURRECTION_DATA_DIR")
        .expect("set RUSTYCORE_RESURRECTION_DATA_DIR to the installed Data directory");
    let locale = "enUS";

    let races_path = Path::new(&data_dir)
        .join("dbc")
        .join(locale)
        .join("ChrRaces.db2");
    let races_reader = Wdc4Reader::open(&races_path).expect("open ChrRaces.db2");
    let race_sickness_rows: Vec<_> = races_reader
        .iter_records()
        .map(|(race_id, index)| (race_id, races_reader.get_field_i32(index, 20)))
        .collect();
    let races = crate::character_progression::ChrRacesStore::load(&data_dir, locale)
        .expect("load ChrRaces.db2");
    assert_eq!(races.len(), race_sickness_rows.len());
    let mut sickness_races: Vec<_> = race_sickness_rows
        .iter()
        .filter(|(_, spell_id)| *spell_id > 0)
        .map(|(race_id, spell_id)| {
            let race = races
                .get(*race_id)
                .expect("raw ChrRaces row exists in typed store");
            assert_eq!(race.res_sickness_spell_id, *spell_id);
            (*race_id, race.name.as_str(), *spell_id as u32)
        })
        .collect();
    sickness_races.sort_unstable_by_key(|(race_id, _, spell_id)| (*spell_id, *race_id));
    let sickness_spell_ids: BTreeSet<_> = sickness_races
        .iter()
        .map(|(_, _, spell_id)| *spell_id)
        .collect();
    let mut spell_ids = BTreeSet::from(GHOST_WISP_SPELL_IDS);
    spell_ids.extend(sickness_spell_ids.iter().copied());

    println!(
        "# tableHash\tChrRaces.db2\t0x{:08X}\theaderRecords={}\tloadedRecords={}",
        races_reader.table_hash(),
        races_reader.record_count(),
        races.len(),
    );
    println!(
        "# selectedSickness\traces={}\tuniqueSpellIDs={}",
        sickness_races.len(),
        sickness_spell_ids.len(),
    );
    println!("ChrRaces\tRaceID\tName\tResSicknessSpellID");
    for (race_id, name, spell_id) in sickness_races {
        println!("ChrRaces\t{race_id}\t{name}\t{spell_id}");
    }

    let liquid_path = Path::new(&data_dir)
        .join("dbc")
        .join(locale)
        .join("LiquidType.db2");
    let liquid_reader = Wdc4Reader::open(&liquid_path).expect("open LiquidType.db2");
    let liquids = crate::LiquidTypeStore::load(&data_dir, locale).expect("load LiquidType.db2");
    assert_eq!(liquids.len(), liquid_reader.iter_records().count());
    let mut liquid_rows: Vec<_> = liquids
        .entries()
        .filter(|row| GHOST_WISP_SPELL_IDS.contains(&row.spell_id))
        .collect();
    liquid_rows.sort_unstable_by_key(|row| row.id);
    println!(
        "# tableHash\tLiquidType.db2\t0x{:08X}\theaderRecords={}\tloadedRecords={}\tselectedSpellRows={}",
        liquid_reader.table_hash(),
        liquid_reader.record_count(),
        liquids.len(),
        liquid_rows.len(),
    );
    println!("LiquidType\tRecordID\tSpellID\tName");
    for row in liquid_rows {
        println!("LiquidType\t{}\t{}\t{}", row.id, row.spell_id, row.name);
    }

    let item_effect_path = Path::new(&data_dir)
        .join("dbc")
        .join(locale)
        .join("ItemEffect.db2");
    let item_effect_reader = Wdc4Reader::open(&item_effect_path).expect("open ItemEffect.db2");
    let item_effects =
        crate::ItemEffectStore::load(&data_dir, locale).expect("load ItemEffect.db2");
    assert_eq!(
        item_effects.len(),
        item_effect_reader.iter_records().count()
    );
    let mut selected_item_effects: Vec<_> = item_effects
        .values()
        .filter(|row| row.trigger_type == 5 && row.spell_id > 0)
        .collect();
    selected_item_effects.sort_unstable_by_key(|row| (row.parent_item_id, row.id));
    let pickup_positive_charged_count = selected_item_effects
        .iter()
        .filter(|row| row.trigger_type == 5 && row.spell_id > 0 && row.charges != 0)
        .count();
    let item_path = Path::new(&data_dir)
        .join("dbc")
        .join(locale)
        .join("Item.db2");
    let item_reader = Wdc4Reader::open(&item_path).expect("open Item.db2");
    let items = crate::ItemStore::load(&data_dir, locale).expect("load Item.db2");
    let item_reader_rows: Vec<_> = item_reader.iter_records().collect();
    let loaded_item_rows = item_reader_rows
        .iter()
        .filter(|(item_id, _)| items.get(*item_id).is_some())
        .count();
    let item_sparse_path = Path::new(&data_dir)
        .join("dbc")
        .join(locale)
        .join("ItemSparse.db2");
    let item_sparse_reader = Wdc4Reader::open(&item_sparse_path).expect("open ItemSparse.db2");
    let item_sparse = crate::ItemStatsStore::load(&data_dir, locale)
        .expect("load ItemSparse.db2 through ItemStatsStore");
    let sparse_template_count = item_sparse.sparse_templates_like_cpp().count();
    let mut nonlegacy_pickup_count = 0;
    let mut nonlegacy_charged_pickup_count = 0;
    for row in &selected_item_effects {
        let legacy = item_sparse
            .raw_flags(row.parent_item_id)
            .map(|flags| flags[0] & (wow_constants::ItemFlags::LEGACY.bits() as u32) != 0);
        if legacy == Some(false) {
            nonlegacy_pickup_count += 1;
            if row.charges != 0 {
                nonlegacy_charged_pickup_count += 1;
            }
        }
    }
    println!(
        "# tableHash\tItemEffect.db2\t0x{:08X}\theaderRecords={}\tloadedRecords={}\ttriggerType5Positive={}\ttriggerType5PositiveNonzeroCharges={}",
        item_effect_reader.table_hash(),
        item_effect_reader.record_count(),
        item_effects.len(),
        selected_item_effects.len(),
        pickup_positive_charged_count,
    );
    println!(
        "# tableHash\tItem.db2\t0x{:08X}\theaderRecords={}\tdecodedRecords={}\ttypedRowsPresent={}",
        item_reader.table_hash(),
        item_reader.record_count(),
        item_reader_rows.len(),
        loaded_item_rows,
    );
    println!(
        "# tableHash\tItemSparse.db2\t0x{:08X}\theaderRecords={}\tdecodedRecords={}\tsparseTemplateRows={}",
        item_sparse_reader.table_hash(),
        item_sparse_reader.record_count(),
        item_sparse_reader.iter_records().count(),
        sparse_template_count,
    );
    println!(
        "# pickupPositiveReachability\trows={}\titemDb2Present={}\tsparseFlagsPresent={}\tnonLegacy={}\tnonLegacyNonzeroCharges={}\trequiredLevel=unavailable-from-current-ItemSparse-template-reader",
        selected_item_effects.len(),
        selected_item_effects
            .iter()
            .filter(|row| items.get(row.parent_item_id).is_some())
            .count(),
        selected_item_effects
            .iter()
            .filter(|row| item_sparse.raw_flags(row.parent_item_id).is_some())
            .count(),
        nonlegacy_pickup_count,
        nonlegacy_charged_pickup_count,
    );
    println!(
        "ItemEffect\tRecordID\tParentItemID\tLegacySlot\tTriggerType\tSpellID\tCharges\tItemPresent\tItemClass\tItemInventoryType\tSparseFlags0\tLegacy\tAllowableClass\tSparseInventoryType\tRequiredLevel"
    );
    for row in selected_item_effects {
        let item = items.get(row.parent_item_id);
        let sparse = item_sparse.sparse_template(row.parent_item_id);
        let flags = item_sparse.raw_flags(row.parent_item_id);
        let legacy =
            flags.map(|flags| flags[0] & (wow_constants::ItemFlags::LEGACY.bits() as u32) != 0);
        println!(
            "ItemEffect\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\tunavailable",
            row.id,
            row.parent_item_id,
            row.legacy_slot_index,
            row.trigger_type,
            row.spell_id,
            row.charges,
            item.is_some(),
            item.map(|item| item.class_id),
            item.map(|item| item.inventory_type),
            flags.map(|flags| flags[0]),
            legacy,
            sparse.map(|template| template.allowable_class),
            sparse.map(|template| template.inventory_type),
        );
    }

    let effects = SpellEffectDb2Store::load(&data_dir, locale).expect("load SpellEffect.db2");
    let effect_table_hash = effects
        .table_hash_like_cpp()
        .expect("SpellEffect.db2 store has its WDC4 table hash");
    let mut effect_rows: Vec<_> = effects
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    effect_rows
        .sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.effect_index, row.id));
    println!("# tableHash\tSpellEffect.db2\t0x{effect_table_hash:08X}");
    println!("# loadedRows\tSpellEffect.db2\t{}", effects.len());
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
    for spell_id in &spell_ids {
        println!(
            "# SpellEffectCount\tspell={spell_id}\trows={}",
            effect_rows
                .iter()
                .filter(|row| row.spell_id == *spell_id)
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
    println!("# loadedRows\tSpellMisc.db2\t{}", misc.len());
    println!("# SpellMiscSelected\tselectionDifficulty=0\tSpellID\tRecordID");
    for spell_id in &spell_ids {
        let selected = misc
            .entry_for_spell_difficulty_like_cpp(*spell_id, 0)
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

    let aura_options =
        SpellAuraOptionsStore::load(&data_dir, locale).expect("load SpellAuraOptions.db2");
    let aura_options_table_hash = aura_options
        .table_hash_like_cpp()
        .expect("SpellAuraOptions.db2 store has its WDC4 table hash");
    let mut aura_options_rows: Vec<_> = aura_options
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    aura_options_rows.sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.id));
    println!("# tableHash\tSpellAuraOptions.db2\t0x{aura_options_table_hash:08X}");
    println!("# loadedRows\tSpellAuraOptions.db2\t{}", aura_options.len());
    println!(
        "SpellAuraOptions\tRecordID\tSpellID\tDifficulty\tCumulativeAura\tSelectedForDifficulty0"
    );
    for row in &aura_options_rows {
        let selected = aura_options
            .entry_for_spell_difficulty_like_cpp(row.spell_id, 0)
            .is_some_and(|selected| selected.id == row.id);
        println!(
            "SpellAuraOptions\t{}\t{}\t{}\t{}\t{}",
            row.id, row.spell_id, row.difficulty_id, row.cumulative_aura, selected,
        );
    }
    for spell_id in &spell_ids {
        let selected = aura_options
            .entry_for_spell_difficulty_like_cpp(*spell_id, 0)
            .map(|row| (row.id, row.cumulative_aura));
        println!("# SpellAuraOptionsSelected\t0\t{spell_id}\t{selected:?}");
    }

    let durations = SpellDurationStore::load(&data_dir, locale).expect("load SpellDuration.db2");
    let duration_table_hash = durations
        .table_hash_like_cpp()
        .expect("SpellDuration.db2 store has its WDC4 table hash");
    println!("# tableHash\tSpellDuration.db2\t0x{duration_table_hash:08X}");
    println!("# loadedRows\tSpellDuration.db2\t{}", durations.len());
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

    let visuals =
        SpellXSpellVisualStore::load(&data_dir, locale).expect("load SpellXSpellVisual.db2");
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
    let related_visual_ids: BTreeSet<_> =
        visual_rows.iter().map(|row| row.spell_visual_id).collect();
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
    for spell_id in GHOST_WISP_SPELL_IDS {
        println!(
            "# SpellXSpellVisualCount\tspell={spell_id}\trows={}",
            visual_rows
                .iter()
                .filter(|row| row.spell_id == spell_id)
                .count()
        );
    }

    // Saveability boundary for the selected self-cast auras. In the pinned
    // C++ source `SpellMgr.cpp:3278-3280` marks Aura::CanBeSaved false when
    // `SpellAuraInterruptFlags::LeaveWorld` is present (`SpellDefines.h:92`,
    // bit 0x00080000). This is raw installed DB2 evidence only; SQL/hotfix
    // overlays are intentionally outside this diagnostic.
    let interrupts =
        SpellInterruptsStore::load(&data_dir, locale).expect("load SpellInterrupts.db2");
    let interrupt_table_hash = interrupts
        .table_hash_like_cpp()
        .expect("SpellInterrupts.db2 store has its WDC4 table hash");
    let mut interrupt_rows: Vec<_> = interrupts
        .entries_like_cpp()
        .filter(|row| spell_ids.contains(&row.spell_id))
        .collect();
    interrupt_rows.sort_unstable_by_key(|row| (row.spell_id, row.difficulty_id, row.id));
    println!("# tableHash\tSpellInterrupts.db2\t0x{interrupt_table_hash:08X}");
    println!(
        "SpellInterrupts\tRecordID\tSpellID\tDifficulty\tInterruptFlags\tAuraInterruptFlags[0]\tAuraInterruptFlags[1]\tChannelInterruptFlags[0]\tChannelInterruptFlags[1]\tLeaveWorld"
    );
    for row in &interrupt_rows {
        let aura_flags = row.aura_interrupt_flags.map(|flags| flags as u32);
        let leave_world = aura_flags[0] & 0x0008_0000 != 0;
        println!(
            "SpellInterrupts\t{}\t{}\t{}\t0x{:04X}\t0x{:08X}\t0x{:08X}\t0x{:08X}\t0x{:08X}\t{}",
            row.id,
            row.spell_id,
            row.difficulty_id,
            row.interrupt_flags as u16,
            aura_flags[0],
            aura_flags[1],
            row.channel_interrupt_flags[0] as u32,
            row.channel_interrupt_flags[1] as u32,
            leave_world,
        );
    }
    for spell_id in GHOST_WISP_SPELL_IDS {
        println!(
            "# SpellInterruptsCount\tspell={spell_id}\trows={}\tleaveWorldRows={}",
            interrupt_rows
                .iter()
                .filter(|row| row.spell_id == spell_id)
                .count(),
            interrupt_rows
                .iter()
                .filter(|row| {
                    row.spell_id == spell_id
                        && row.aura_interrupt_flags[0] as u32 & 0x0008_0000 != 0
                })
                .count(),
        );
    }
}
