use super::SpellTable;
use crate::wdc4::creation::{CreationDb2, CreationTable};
mod custom_sources;
mod fixtures;
mod value_inputs;
use fixtures::{cell, empty_file, reader, regular_file};

#[test]
fn regular_storage_bound_includes_unresolved_copies_not_header_maximum_or_only_live_rows() {
    let table = SpellTable::RandPropPoints;
    let mut value = reader(table);
    value.copy_table = vec![(10, 7), (99, 777)];
    value.header.max_id = 1024;
    let value = CreationDb2::checked(value, CreationTable::Spell(table)).unwrap();
    assert_eq!(value.ids().collect::<Vec<_>>(), [7, 10]);
    assert_eq!(value.storage_last_index(), 99);
}

#[test]
fn all_spell_schemas_enforce_native_types_hash_layout_parent_and_locale() {
    for table in SpellTable::ALL {
        assert!(CreationDb2::checked(reader(table), CreationTable::Spell(table)).is_ok());
        for mutate in [(152, 0), (156, 1), (168, 2), (172, 3), (176, 4), (184, 5)] {
            let mut value = reader(table);
            match mutate.1 {
                0 => value.header.table_hash ^= 1,
                1 => value.header._layout_hash ^= 1,
                2 => value.header._locale ^= 1,
                3 => value.header.flags ^= 1,
                4 => value.header.total_field_count ^= 1,
                5 => value.header._parent_lookup_count ^= 1,
                _ => unreachable!(),
            }
            assert!(
                CreationDb2::checked(value, CreationTable::Spell(table)).is_err(),
                "{} field {}",
                table.name(),
                mutate.0
            );
        }
    }
}

#[test]
fn spell_misc_has_seventeen_words_and_retains_signed_float_and_unsigned_short_bits() {
    let table = SpellTable::SpellMisc;
    let mut value = reader(table);
    for i in 0..17 {
        cell(&mut value, 0, i, 32, 0x8000_0000 | i as u32);
    }
    cell(&mut value, 1, 0, 16, 0xFFFF);
    cell(&mut value, 2, 0, 16, 0xFFFF);
    cell(&mut value, 7, 0, 32, 0x7FC0_1234);
    let value = CreationDb2::checked(value, CreationTable::Spell(table)).unwrap();
    for i in 0..17 {
        assert_eq!(value.bits(7, 0, i).unwrap(), 0x8000_0000 | i as u32);
    }
    assert!(value.bits(7, 0, 17).is_err());
    assert_eq!(value.bits(7, 1, 0).unwrap() as i16, -1);
    assert_eq!(value.bits(7, 2, 0).unwrap() as u16, u16::MAX);
    assert_eq!(
        f32::from_bits(value.bits(7, 7, 0).unwrap()).to_bits(),
        0x7FC0_1234
    );
    assert_eq!(value.bits(7, 16, 0).unwrap(), 0); // genuine extra-parent zero initialization
}

#[test]
fn spell_effect_masks_targets_and_extra_parent_keep_source_widths() {
    let table = SpellTable::SpellEffect;
    let mut value = reader(table);
    for i in 0..4 {
        cell(&mut value, 27, i, 32, 0x8000_0000 | i as u32);
    }
    cell(&mut value, 28, 0, 16, 0x8000);
    cell(&mut value, 28, 1, 16, 0xFFFF);
    value.relationship_ids[0] = Some(0xFFFF_FFFE);
    let value = CreationDb2::checked(value, CreationTable::Spell(table)).unwrap();
    for i in 0..4 {
        assert_eq!(value.bits(7, 27, i).unwrap(), 0x8000_0000 | i as u32);
    }
    assert_eq!(value.bits(7, 28, 0).unwrap() as i16, i16::MIN);
    assert_eq!(value.bits(7, 28, 1).unwrap() as i16, -1);
    assert_eq!(value.bits(7, 29, 0).unwrap(), 0xFFFF_FFFE);
}

#[test]
fn in_record_learn_parent_override_and_spell_copy_order_match_source() {
    let table = SpellTable::SpellLearnSpell;
    let mut value = reader(table);
    cell(&mut value, 0, 0, 32, 42);
    value.relationship_ids[0] = Some(77);
    value.copy_table = vec![(10, 7), (11, 10), (7, 11), (12, 0), (13, 999)];
    let value = CreationDb2::checked(value, CreationTable::Spell(table)).unwrap();
    assert_eq!(value.ids().collect::<Vec<_>>(), [7, 10, 11]);
    assert_eq!(value.bits(11, 0, 0).unwrap(), 77);
    let table = SpellTable::SpellEmpower;
    let mut value = reader(table);
    value.copy_table = vec![(10, 7), (11, 10)];
    let value = CreationDb2::checked(value, CreationTable::Spell(table)).unwrap();
    assert_eq!(value.bits(11, 0, 0).unwrap(), 11); // copy's new inline ID, not source 7
    let mut invalid = reader(table);
    invalid.copy_table = vec![(1025, 7)];
    assert!(CreationDb2::checked(invalid, CreationTable::Spell(table)).is_err());
}

#[test]
fn readable_spell_text_uses_full_logical_record_extent_without_unknown_zero_fill() {
    let table = SpellTable::SpellName;
    let make = || {
        let mut value = reader(table);
        value.header.record_count = 2; // source section zero one, unknown section one
        value.header.string_table_size = 9; // first pool five + unknown pool four
        cell(&mut value, 0, 0, 32, 8); // relative to full logical two*four records
        value.string_tables = vec![b"a\xFFb\0\0".to_vec()];
        value.record_string_table_indices = vec![Some(0)];
        value
    };
    let value = CreationDb2::checked(make(), CreationTable::Spell(table)).unwrap();
    assert_eq!(value.spell_text(7, 0).unwrap(), b"a\xFFb");
    for displacement in [4, 13, 17, u32::MAX] {
        let mut invalid = make();
        cell(&mut invalid, 0, 0, 32, displacement);
        let value = CreationDb2::checked(invalid, CreationTable::Spell(table)).unwrap();
        assert!(value.spell_text(7, 0).is_err());
    }
    let mut null = make();
    cell(&mut null, 0, 0, 32, 0);
    let value = CreationDb2::checked(null, CreationTable::Spell(table)).unwrap();
    assert!(value.spell_text(7, 0).unwrap().is_empty());
}

#[test]
fn empty_spell_files_require_primitive_metadata_and_never_fall_back_to_available_files() {
    let directory =
        std::env::temp_dir().join(format!("forever-empty-spells-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    for table in [
        SpellTable::SpellEmpower,
        SpellTable::SpellEmpowerStage,
        SpellTable::SpellPowerDifficulty,
        SpellTable::SpellReagentsCurrency,
        SpellTable::SpellScaling,
        SpellTable::SpellProcsPerMinuteMod,
    ] {
        let path = directory.join(format!("{}.db2", table.name()));
        let bytes = empty_file(table);
        std::fs::write(&path, &bytes).unwrap();
        let (value, unknown) = CreationDb2::open_spell(&directory, table, false).unwrap();
        assert_eq!(value.ids().count(), 0);
        assert_eq!(unknown, 0);
        std::fs::write(&path, &bytes[..204]).unwrap();
        assert!(CreationDb2::open_spell(&directory, table, false).is_err());
        std::fs::remove_file(path).unwrap();
    }
    assert!(CreationDb2::open_spell(&directory, SpellTable::SpellName, true).is_err());
    std::fs::write(
        directory.join("SpellName.available.db2"),
        empty_file(SpellTable::SpellName),
    )
    .unwrap();
    assert!(CreationDb2::open_spell(&directory, SpellTable::SpellName, false).is_err());
    assert!(CreationDb2::open_spell(&directory, SpellTable::SpellName, true).is_err());
    std::fs::remove_file(directory.join("SpellName.available.db2")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn public_raw_spell_batch_decodes_all_forty_nine_source_record_types() {
    use crate::forever_spells::{SpellBaseline, SpellRecords};
    let directory =
        std::env::temp_dir().join(format!("forever-raw-spell-batch-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    for table in SpellTable::ALL {
        let mut value = reader(table);
        custom_sources::prepare(table, &mut value);
        value_inputs::prepare(table, &mut value);
        if table == SpellTable::SpellMisc {
            for index in 0..17 {
                cell(&mut value, 0, index, 32, 0x8000_0000 | index as u32);
            }
            cell(&mut value, 1, 0, 16, 0xFFFF);
            cell(&mut value, 2, 0, 16, 0xFFFF);
            cell(&mut value, 7, 0, 32, 0x7FC0_1234);
        }
        if table == SpellTable::SpellEffect {
            for index in 0..4 {
                cell(&mut value, 27, index, 32, 0x8000_0000 | index as u32);
            }
            cell(&mut value, 28, 0, 16, 0x8000);
            cell(&mut value, 28, 1, 16, 0xFFFF);
        }
        if table == SpellTable::SpellReagents {
            for index in 0..8 {
                cell(&mut value, 1, index, 32, u32::MAX - index as u32);
                cell(&mut value, 2, index, 16, 0xFFFF - index as u32);
                cell(&mut value, 4, index, 8, 0xFF - index as u32);
            }
        }
        std::fs::write(
            directory.join(format!("{}.db2", table.name())),
            regular_file(table, value),
        )
        .unwrap();
    }
    let batch = SpellRecords::load(&directory, SpellBaseline::Complete).unwrap();
    custom_sources::assert_records(&batch);
    value_inputs::assert_records(&batch);
    for (index, (name, known, unknown)) in batch.counts().into_iter().enumerate() {
        assert_eq!(name, SpellTable::ALL[index].name());
        assert_eq!((known, unknown), (1, 0));
    }
    let misc = &batch.spell_misc[0];
    assert_eq!(misc.attributes[16], 0x8000_0010u32 as i32);
    assert_eq!(misc.difficulty_id, -1);
    assert_eq!(misc.casting_time_index, u16::MAX);
    assert_eq!(misc.speed.to_bits(), 0x7FC0_1234);
    assert_eq!(misc.spell_id, 0);
    assert_eq!(
        batch.spell_effects[0].effect_spell_class_mask[3],
        0x8000_0003
    );
    assert_eq!(batch.spell_effects[0].implicit_target, [i16::MIN, -1]);
    assert_eq!(batch.spell_reagents[0].reagent[7], -8);
    assert_eq!(batch.spell_reagents[0].reagent_count[7], -8);
    assert_eq!(batch.spell_reagents[0].reagent_source[7], 248);
    assert_eq!(batch.spell_names[0].name.at(6), Some([].as_slice()));
    assert_eq!(batch.spell_names[0].name.at(0), None);
    assert_eq!(batch.battle_pet_species[0].id, 7);
    let effective = batch
        .finish(
            SpellRecords::default(),
            SpellRecords::default(),
            6,
            crate::forever_spells::SpellLocaleRecords::default(),
            crate::forever_spells::SpellLocaleRecords::default(),
            &crate::Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    for (name, known, unknown) in effective.counts() {
        assert_eq!((known, unknown), (1, 0), "{name}");
    }
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(
        crate::forever_spells::SPELL_TABLE_HASHES
            .into_iter()
            .map(|hash| (hash, 7, 2)),
    );
    let removed = SpellRecords::load(&directory, SpellBaseline::Complete)
        .unwrap()
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &removals,
        )
        .unwrap();
    assert!(
        removed
            .counts()
            .into_iter()
            .all(|(_, known, unknown)| (known, unknown) == (0, 0))
    );
    // Even failure at the last dependency cannot return a partial batch.
    std::fs::remove_file(directory.join("LiquidType.db2")).unwrap();
    assert!(SpellRecords::load(&directory, SpellBaseline::Complete).is_err());
    for table in SpellTable::ALL.into_iter().take(41) {
        std::fs::remove_file(directory.join(format!("{}.db2", table.name()))).unwrap();
    }
    std::fs::remove_dir(directory).unwrap();
}
