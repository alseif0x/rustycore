use super::*;

fn line(id: u32, parent: u32) -> SkillLineRecord {
    SkillLineRecord {
        id,
        category: 1,
        spell_icon_file: 0,
        can_link: 0,
        parent_skill: parent,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell: 0,
        expansion_name_shared_string: 0,
        horde_expansion_name_shared_string: 0,
    }
}
fn rc(id: u32, skill: u16) -> SkillRaceClassRecord {
    SkillRaceClassRecord {
        id,
        skill,
        class_mask: 0,
        flags: 0,
        availability: 1,
        min_level: 0,
        tier: 0,
        race_mask: 0,
    }
}
fn ability(id: u32, skill: u16, skillup: i16) -> SkillAbilityRecord {
    SkillAbilityRecord {
        id,
        skill_line: skill,
        spell: 1,
        min_skill_rank: 0,
        class_mask: 0,
        supercedes_spell: 0,
        acquire_method: 1,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 0,
        unique_bit: 0,
        trade_skill_category: 0,
        skillup_skill_line: skillup,
        field_5_5_4_67090_014: [0; 2],
        race_mask: 0,
    }
}
fn loadout(id: u32) -> LoadoutRecord {
    LoadoutRecord {
        id,
        class: 1,
        purpose: 9,
        item_context: 0,
        field_1_60_1_69876_003: 0,
        race_mask: 1,
    }
}
fn item(id: u32, loadout: u16, item: u32) -> LoadoutItemRecord {
    LoadoutItemRecord { id, loadout, item }
}
fn removals(rows: impl IntoIterator<Item = (u32, i32, u8)>) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(rows)
}
fn records() -> BirthRecords {
    BirthRecords {
        skill_lines: vec![line(1, 0)],
        race_class: vec![rc(1, 1)],
        abilities: vec![ability(1, 1, 0)],
        loadouts: vec![loadout(1)],
        loadout_items: vec![item(1, 1, 7)],
        unknown_ability_records: 5,
    }
}

#[test]
fn all_five_stores_compose_official_then_custom_before_indexes() {
    let mut official = records();
    official.skill_lines[0].parent_skill = 8;
    official.race_class[0].skill = 8;
    official.abilities[0].skillup_skill_line = 8;
    official.loadouts[0].class = 8;
    official.loadout_items[0].loadout = 8;
    let mut custom = records();
    custom.skill_lines[0].parent_skill = 9;
    custom.race_class[0].skill = 9;
    custom.abilities[0].skillup_skill_line = 9;
    custom.loadouts[0].class = 9;
    custom.loadout_items[0].loadout = 9;
    let catalog = records().finish(official, custom, &removals([])).unwrap();
    assert_eq!(catalog.skill_line(1).unwrap().parent_skill, 9);
    assert_eq!(catalog.race_class_records().next().unwrap().skill, 9);
    assert_eq!(catalog.abilities_for_skill(9).next().unwrap().id, 1);
    assert_eq!(catalog.loadouts().next().unwrap().class, 9);
    assert_eq!(catalog.items_for_loadout(9).next().unwrap().item, 7);
    assert_eq!(catalog.child_lines(8).count(), 0);
    assert_eq!(catalog.abilities_for_skill(8).count(), 0);
    assert_eq!(catalog.items_for_loadout(8).count(), 0);
    assert_eq!(catalog.counts(), [1, 1, 1, 1, 1, 5]);
}

#[test]
fn final_removals_apply_to_every_table_even_after_custom_addition() {
    let deleted = removals(
        [
            SKILL_LINE_HASH,
            RACE_CLASS_HASH,
            ABILITY_HASH,
            LOADOUT_HASH,
            LOADOUT_ITEM_HASH,
        ]
        .into_iter()
        .map(|hash| (hash, 1, 2)),
    );
    let catalog = records().finish(records(), records(), &deleted).unwrap();
    assert_eq!(catalog.counts(), [0, 0, 0, 0, 0, 5]);
    assert!(catalog.skill_line(1).is_none());
    assert_eq!(catalog.child_lines(0).count(), 0);
    assert_eq!(catalog.abilities_for_skill(1).count(), 0);
    assert_eq!(catalog.items_for_loadout(1).count(), 0);
}

#[test]
fn source_overlay_duplicate_ids_overwrite_in_received_order_not_fail_or_sort() {
    let official = BirthRecords {
        skill_lines: vec![line(1, 2), line(1, 3), line(2, 4), line(2, 5)],
        ..Default::default()
    };
    let custom = BirthRecords {
        skill_lines: vec![line(1, 6), line(1, 7)],
        ..Default::default()
    };
    let catalog = records().finish(official, custom, &removals([])).unwrap();
    assert_eq!(catalog.skill_line(1).unwrap().parent_skill, 7);
    assert_eq!(catalog.skill_line(2).unwrap().parent_skill, 5);
    let mut corrupt = records();
    corrupt.skill_lines.push(line(1, 2));
    assert!(
        corrupt
            .finish(Default::default(), Default::default(), &removals([]))
            .is_err()
    );
}

#[test]
fn skillup_precedence_signed_conversion_and_storage_vector_order_are_retained() {
    let baseline = BirthRecords {
        abilities: vec![
            ability(3, 7, 8),
            ability(2, 7, 0),
            ability(1, 7, 8),
            ability(4, 7, -1),
        ],
        ..Default::default()
    };
    let catalog = baseline
        .finish(Default::default(), Default::default(), &removals([]))
        .unwrap();
    assert_eq!(
        catalog
            .abilities_for_skill(8)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(
        catalog
            .abilities_for_skill(7)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(
        catalog
            .abilities_for_skill(u32::MAX)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [4]
    );
}

#[test]
fn parent_and_item_indexes_use_final_rows_keep_source_order_and_do_not_deduplicate_relations() {
    let baseline = BirthRecords {
        skill_lines: vec![line(3, 5), line(1, 5), line(2, 0)],
        loadout_items: vec![item(3, 5, 7), item(1, 5, 7), item(2, 8, 9)],
        ..Default::default()
    };
    let custom = BirthRecords {
        skill_lines: vec![line(3, 8)],
        loadout_items: vec![item(2, 5, 10)],
        ..Default::default()
    };
    let catalog = baseline
        .finish(Default::default(), custom, &removals([]))
        .unwrap();
    assert_eq!(
        catalog.child_lines(5).map(|row| row.id).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        catalog.child_lines(8).map(|row| row.id).collect::<Vec<_>>(),
        [3]
    );
    assert_eq!(catalog.child_lines(0).count(), 0);
    assert_eq!(
        catalog
            .items_for_loadout(5)
            .map(|row| row.item)
            .collect::<Vec<_>>(),
        [7, 10, 7]
    );
    assert_eq!(catalog.items_for_loadout(8).count(), 0);
}

#[test]
fn removed_ids_preserve_unsigned_bits_and_later_valid_cancels_removal() {
    let baseline = BirthRecords {
        skill_lines: vec![line(u32::MAX, 0), line(1, 0)],
        ..Default::default()
    };
    let statuses = removals([
        (SKILL_LINE_HASH, -1, 2),
        (SKILL_LINE_HASH, 1, 2),
        (SKILL_LINE_HASH, 1, 1),
    ]);
    let catalog = baseline
        .finish(Default::default(), Default::default(), &statuses)
        .unwrap();
    assert!(catalog.skill_line(u32::MAX).is_none());
    assert!(catalog.skill_line(1).is_some());
}

#[test]
fn race_class_iteration_is_db2_id_order_not_claimed_multimap_match_order() {
    let baseline = BirthRecords {
        race_class: vec![rc(3, 7), rc(1, 7), rc(2, 99)],
        ..Default::default()
    };
    let catalog = baseline
        .finish(Default::default(), Default::default(), &removals([]))
        .unwrap();
    assert_eq!(
        catalog
            .race_class_records()
            .map(|row| (row.id, row.skill))
            .collect::<Vec<_>>(),
        [(1, 7), (2, 99), (3, 7)]
    );
}

#[test]
fn unknown_baseline_count_is_not_erased_by_overlay_metadata_or_effective_additions() {
    let mut baseline = records();
    baseline.abilities.clear();
    let mut official = records();
    official.unknown_ability_records = 0;
    let catalog = baseline
        .finish(official, Default::default(), &removals([]))
        .unwrap();
    assert_eq!(catalog.counts()[2], 1);
    assert_eq!(catalog.counts()[5], 5);
}
