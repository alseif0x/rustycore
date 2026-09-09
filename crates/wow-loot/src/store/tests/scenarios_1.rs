//! Loot store definitions and templates regression scenarios, part 1 of 2.
//!
//! Moved out of the lib.rs root under #642; every test is unchanged.

use super::*;

#[test]
fn loot_store_item_validity_matches_cpp_basic_guards() {
    assert!(item(25, 0, 100.0, 0).is_valid_like_cpp(true));
    assert!(!item(0, 0, 100.0, 0).is_valid_like_cpp(true));
    assert!(!item(25, 0, 100.0, 0).is_valid_like_cpp(false));
    assert!(!item(25, 0, 0.0, 0).is_valid_like_cpp(true));
    assert!(item(25, 0, 0.0, 1).is_valid_like_cpp(true));
    assert!(!item(25, 0, 0.0000001, 0).is_valid_like_cpp(true));
    assert!(item(0, 700, 25.0, 0).is_valid_like_cpp(true));
    assert!(!item(0, 700, 0.0, 0).is_valid_like_cpp(true));
    assert!(
        LootStoreItem {
            needs_quest: true,
            ..item(0, 700, 0.0, 0)
        }
        .is_valid_like_cpp(true)
    );
}

#[test]
fn loot_conditions_else_group_negative_and_unsupported_rows_match_cpp_shape() {
    let conditions = vec![
        condition(0, 25, 100, false),
        condition(0, 25, 200, false),
        condition(1, 25, 300, true),
    ];

    assert!(loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 100 || condition.value1 == 200),
    ));
    assert!(loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 100),
    ));
    assert!(!loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 300),
    ));

    let mut scripted = condition(0, 25, 100, false);
    scripted.script_name = "Unsupported".into();
    assert!(!loot_conditions_allow_player_like_cpp_representable(
        &[scripted],
        |_| Some(true),
    ));
    assert!(loot_conditions_allow_player_like_cpp_representable(
        &[condition(0, -1, 100, false)],
        |_| Some(true),
    ));
}

#[test]
fn loot_conditions_else_group_store_does_not_require_contiguous_rows_like_cpp() {
    let conditions = vec![
        condition(0, 25, 100, false),
        condition(1, 25, 300, false),
        condition(0, 25, 200, false),
    ];

    assert!(loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 100 || condition.value1 == 200),
    ));
    assert!(!loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 100),
    ));
}

#[test]
fn loot_conditions_reference_templates_expand_like_cpp() {
    let conditions = vec![condition(0, -42, 0, false), condition(1, 25, 300, false)];
    let mut references = HashMap::new();
    references.insert(42, vec![condition(0, 25, 100, false)]);

    assert!(
        loot_conditions_allow_player_with_references_like_cpp_representable(
            &conditions,
            &references,
            |condition| Some(condition.value1 == 100),
        )
    );
    assert!(
        !loot_conditions_allow_player_with_references_like_cpp_representable(
            &conditions,
            &references,
            |condition| Some(condition.value1 == 200),
        )
    );
}

#[test]
fn loot_condition_reference_ids_extract_negative_condition_types_like_cpp() {
    assert_eq!(
        loot_condition_reference_ids_like_cpp(&[
            condition(0, -42, 0, false),
            condition(0, 25, 0, false),
            condition(0, -900, 0, false),
        ]),
        vec![42, 900]
    );
}

#[test]
fn loot_condition_reference_self_reference_guard_matches_cpp_load_skip() {
    assert!(loot_condition_reference_self_references_like_cpp(-42, -42));
    assert!(!loot_condition_reference_self_references_like_cpp(1, -42));
    assert!(!loot_condition_reference_self_references_like_cpp(
        -42, -900
    ));
    assert!(!loot_condition_reference_self_references_like_cpp(-42, 25));
}

#[test]
fn loot_condition_row_loadable_guard_matches_cpp_deterministic_load_skips() {
    assert!(
        loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(0, 0, 0, false))
    );

    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 59, 0, false
        ))
    );
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 19, 0, false
        ))
    );

    let mut invalid_target = condition(0, 25, 1, false);
    invalid_target.condition_target = 1;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_target,));

    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 2, 6948, false
        ))
    );
    let mut valid_item = condition(0, 2, 6948, false);
    valid_item.value2 = 1;
    assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_item));

    assert!(
        loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 6, 469, false
        ))
    );
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 6, 1, false
        ))
    );

    let mut invalid_class = condition(0, 15, 1, false);
    invalid_class.value1 = 1 << 13;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_class,));
    let mut valid_class = condition(0, 15, 1, false);
    valid_class.value1 = 1 << 12;
    assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_class));

    let mut invalid_race = condition(0, 16, 1, false);
    invalid_race.value1 = 1 << 22;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_race,));
    let mut valid_remapped_race = condition(0, 16, 1, false);
    valid_remapped_race.value1 = 1 << 11;
    assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_remapped_race,));

    let mut invalid_level = condition(0, 27, 80, false);
    invalid_level.value2 = 5;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_level,));

    let mut invalid_drunkenstate = condition(0, 10, 4, false);
    invalid_drunkenstate.value1 = 4;
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_drunkenstate,)
    );

    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 35, 0, false
        ))
    );

    let mut invalid_hp_val = condition(0, 37, 1000, false);
    invalid_hp_val.value2 = 5;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_hp_val,));
    let mut invalid_hp_pct_value = condition(0, 38, 101, false);
    invalid_hp_pct_value.value1 = 101;
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_hp_pct_value,)
    );
    let mut invalid_hp_pct_compare = condition(0, 38, 50, false);
    invalid_hp_pct_compare.value2 = 5;
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_hp_pct_compare,)
    );

    let mut instance_guid_data = condition(0, 13, 1, false);
    instance_guid_data.value3 = 1;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&instance_guid_data,));

    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 33, 0, false
        ))
    );
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 34, 0, false
        ))
    );

    let mut invalid_stand_state_mode = condition(0, 42, 2, false);
    invalid_stand_state_mode.value1 = 2;
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_stand_state_mode,)
    );
    let mut invalid_stand_state_value = condition(0, 42, 0, false);
    invalid_stand_state_value.value2 = 10;
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(
            &invalid_stand_state_value,
        )
    );

    let mut invalid_pet_type = condition(0, 45, 16, false);
    invalid_pet_type.value1 = 16;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_pet_type,));

    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
            0, 52, 0, false
        ))
    );
    let mut invalid_type_mask = condition(0, 52, 0x02, false);
    invalid_type_mask.value1 = 0x02;
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_type_mask,));
    let mut valid_type_mask = condition(0, 52, 0x40, false);
    valid_type_mask.value1 = 0x40;
    assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_type_mask));

    let mut invalid_quest_state = condition(0, 47, 1, false);
    invalid_quest_state.value2 = 128;
    assert!(
        !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_quest_state,)
    );

    let mut reference_with_useless_target = condition(0, -42, 0, false);
    reference_with_useless_target.condition_target = 1;
    assert!(
        loot_condition_row_is_loadable_without_external_stores_like_cpp(
            &reference_with_useless_target,
        )
    );
}

#[test]
fn loot_condition_legacy_object_and_type_mask_rows_normalize_like_cpp_load() {
    let legacy_player_object = condition(0, 31, 4, false);
    let normalized =
        loot_condition_row_normalize_without_external_stores_like_cpp(legacy_player_object)
            .unwrap();
    assert_eq!(normalized.condition_type_or_reference, 51);
    assert_eq!(normalized.value1, 6);

    let legacy_player_mask = condition(0, 32, 1 << 4, false);
    let normalized =
        loot_condition_row_normalize_without_external_stores_like_cpp(legacy_player_mask).unwrap();
    assert_eq!(normalized.condition_type_or_reference, 52);
    assert_eq!(normalized.value1, 0x40);

    let legacy_item_object = condition(0, 31, 1, false);
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&legacy_item_object));
    assert!(
        loot_condition_row_normalize_without_external_stores_like_cpp(legacy_item_object).is_none()
    );

    let invalid_object_type = condition(0, 51, 7, false);
    assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_object_type));
}

#[test]
fn condition_compare_matches_cpp_compare_values_order() {
    assert_eq!(condition_compare_values_like_cpp(0, 10, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(1, 11, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(2, 9, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(3, 10, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(4, 10, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(5, 10, 10), None);
}

#[test]
fn loot_template_add_entry_matches_cpp_group_split() {
    let mut template = LootTemplate::default();
    template.add_entry_like_cpp(item(25, 0, 100.0, 0));
    template.add_entry_like_cpp(item(26, 0, 0.0, 2));
    template.add_entry_like_cpp(item(0, 900, 50.0, 2));

    assert_eq!(template.entries().len(), 2);
    assert_eq!(template.groups().len(), 2);
    assert!(template.groups()[0].equal_chanced().is_empty());
    assert_eq!(template.groups()[1].equal_chanced().len(), 1);
    assert_eq!(template.entries()[1].reference, 900);
}

#[test]
fn loot_store_definitions_match_cpp_globals() {
    let definitions: Vec<_> = LootStoreKind::ALL_LIKE_CPP
        .iter()
        .map(|kind| kind.definition_like_cpp())
        .collect();

    assert_eq!(definitions.len(), 12);
    assert_eq!(definitions[0].table_name, "creature_loot_template");
    assert_eq!(definitions[0].entry_name, "creature entry");
    assert!(definitions[0].rates_allowed);
    assert_eq!(definitions[1].table_name, "disenchant_loot_template");
    assert_eq!(definitions[9].table_name, "reference_loot_template");
    assert!(!definitions[9].rates_allowed);
    assert_eq!(definitions[11].table_name, "spell_loot_template");
    assert!(!definitions[11].rates_allowed);
}

#[test]
fn loot_store_load_rows_matches_cpp_clear_validate_and_collect_shape() {
    let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Disenchant);
    let rows = [
        LootTemplateRow {
            entry: 100,
            item: item(25, 0, 100.0, 0),
        },
        LootTemplateRow {
            entry: 100,
            item: item(26, 0, 0.0, 2),
        },
        LootTemplateRow {
            entry: 101,
            item: item(0, 700, 50.0, 0),
        },
        LootTemplateRow {
            entry: 102,
            item: item(0, 0, 100.0, 0),
        },
    ];

    let loaded = store
        .load_rows_like_cpp(rows, |item_id| item_id == 25 || item_id == 26)
        .unwrap();

    assert_eq!(loaded, 3);
    assert!(store.have_loot_for(100));
    assert!(store.have_loot_for(101));
    assert!(!store.have_loot_for(102));
    assert_eq!(store.get_loot_for(100).unwrap().entries().len(), 1);
    assert_eq!(
        store.get_loot_for(100).unwrap().groups()[1]
            .equal_chanced()
            .len(),
        1
    );
    assert_eq!(store.collect_loot_ids_like_cpp().len(), 2);
}

#[test]
fn loot_store_load_rows_rejects_cpp_invalid_group_id() {
    let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
    let err = store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 100,
                item: LootStoreItem {
                    group_id: 128,
                    ..item(25, 0, 100.0, 0)
                },
            }],
            |_| true,
        )
        .unwrap_err();

    assert_eq!(
        err,
        LootStoreLoadError::InvalidGroupId {
            table_name: "item_loot_template",
            entry: 100,
            item_id: 25,
            group_id: 128,
        }
    );
}

#[test]
fn loot_reference_check_matches_cpp_used_missing_and_unused_refs() {
    let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference
        .load_rows_like_cpp(
            [
                LootTemplateRow {
                    entry: 700,
                    item: item(25, 0, 100.0, 0),
                },
                LootTemplateRow {
                    entry: 701,
                    item: item(26, 0, 100.0, 0),
                },
            ],
            |_| true,
        )
        .unwrap();

    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature
        .load_rows_like_cpp(
            [
                LootTemplateRow {
                    entry: 100,
                    item: item(0, 700, 50.0, 0),
                },
                LootTemplateRow {
                    entry: 101,
                    item: item(0, 999, 25.0, 0),
                },
            ],
            |_| true,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Reference, reference);
    stores.insert(LootStoreKind::Creature, creature);

    assert_eq!(
        check_loot_references_like_cpp(&stores),
        LootReferenceCheckReport {
            missing_references: vec![LootReferenceUse {
                store_kind: LootStoreKind::Creature,
                entry: 101,
                item_id: 0,
                reference: 999,
            }],
            unused_reference_ids: vec![701],
        }
    );
}

#[test]
fn loot_reference_check_includes_reference_store_self_refs_like_cpp() {
    let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference
        .load_rows_like_cpp(
            [
                LootTemplateRow {
                    entry: 700,
                    item: item(0, 701, 100.0, 0),
                },
                LootTemplateRow {
                    entry: 701,
                    item: item(25, 0, 100.0, 0),
                },
            ],
            |_| true,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Reference, reference);

    assert_eq!(
        check_loot_references_like_cpp(&stores),
        LootReferenceCheckReport {
            missing_references: Vec::new(),
            unused_reference_ids: vec![700],
        }
    );
}

#[test]
fn loot_condition_link_check_matches_cpp_source_and_item_guards() {
    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature
        .load_rows_like_cpp(
            [
                LootTemplateRow {
                    entry: 100,
                    item: item(25, 0, 100.0, 0),
                },
                LootTemplateRow {
                    entry: 100,
                    item: item(26, 0, 0.0, 2),
                },
                LootTemplateRow {
                    entry: 100,
                    item: item(0, 700, 50.0, 0),
                },
            ],
            |item_id| item_id == 25 || item_id == 26,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature);

    let report = check_loot_condition_links_like_cpp(
        &stores,
        [
            LootConditionId {
                source_type: 1,
                source_group: 100,
                source_entry: 25,
            },
            LootConditionId {
                source_type: 1,
                source_group: 100,
                source_entry: 26,
            },
            LootConditionId {
                source_type: 1,
                source_group: 100,
                source_entry: 0,
            },
            LootConditionId {
                source_type: 1,
                source_group: 999,
                source_entry: 25,
            },
            LootConditionId {
                source_type: 1,
                source_group: 100,
                source_entry: 27,
            },
            LootConditionId {
                source_type: 1,
                source_group: 100,
                source_entry: 28,
            },
            LootConditionId {
                source_type: 99,
                source_group: 100,
                source_entry: 25,
            },
        ],
        |item_id| matches!(item_id, 25 | 26 | 28),
    );

    assert_eq!(
        report,
        LootConditionLinkReport {
            linked: 3,
            unsupported_source_types: vec![LootConditionId {
                source_type: 99,
                source_group: 100,
                source_entry: 25,
            }],
            missing_templates: vec![MissingLootConditionTemplate {
                condition_id: LootConditionId {
                    source_type: 1,
                    source_group: 999,
                    source_entry: 25,
                },
                store_kind: LootStoreKind::Creature,
            }],
            missing_item_templates: vec![MissingLootConditionItemTemplate {
                condition_id: LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 27,
                },
                store_kind: LootStoreKind::Creature,
            }],
            missing_template_items: vec![MissingLootConditionTemplateItem {
                condition_id: LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 28,
                },
                store_kind: LootStoreKind::Creature,
            }],
            missing_reference_templates: Vec::new(),
        }
    );
}

#[test]
fn loot_condition_reference_check_reports_missing_templates_like_cpp_load_guard() {
    let mut report = LootConditionLinkReport {
        linked: 0,
        unsupported_source_types: Vec::new(),
        missing_templates: Vec::new(),
        missing_item_templates: Vec::new(),
        missing_template_items: Vec::new(),
        missing_reference_templates: Vec::new(),
    };

    let uses = [
        LootConditionReferenceUseLikeCpp {
            condition_id: LootConditionId {
                source_type: 1,
                source_group: 100,
                source_entry: 25,
            },
            reference_id: 42,
        },
        LootConditionReferenceUseLikeCpp {
            condition_id: LootConditionId {
                source_type: 5,
                source_group: 200,
                source_entry: 26,
            },
            reference_id: 77,
        },
    ];
    check_loot_condition_references_like_cpp(&mut report, uses, [42]);

    assert_eq!(
        report.missing_reference_templates,
        vec![LootConditionReferenceUseLikeCpp {
            condition_id: LootConditionId {
                source_type: 5,
                source_group: 200,
                source_entry: 26,
            },
            reference_id: 77,
        }]
    );
    assert!(!report.is_clean());
}

#[test]
fn fill_loot_processes_plain_group_and_reference_entries_like_cpp() {
    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature
        .load_rows_like_cpp(
            [
                LootTemplateRow {
                    entry: 100,
                    item: item(25, 0, 100.0, 0),
                },
                LootTemplateRow {
                    entry: 100,
                    item: item(26, 0, 0.0, 1),
                },
                LootTemplateRow {
                    entry: 100,
                    item: item(0, 700, 100.0, 0),
                },
            ],
            |item_id| matches!(item_id, 25 | 26),
        )
        .unwrap();

    let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 700,
                item: item(27, 0, 100.0, 0),
            }],
            |item_id| item_id == 27,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature);
    stores.insert(LootStoreKind::Reference, reference);

    let mut rng = StdRng::seed_from_u64(7);
    let generated = stores[&LootStoreKind::Creature]
        .fill_loot_like_cpp(
            100,
            LootStoreKind::Creature,
            &stores,
            LootFillOptions::default(),
            &mut rng,
            |_| Some(item_metadata(20)),
            |_| 1.0,
            |_| true,
            random_properties,
        )
        .unwrap();

    assert_eq!(
        generated,
        vec![
            generated_item(item(25, 0, 100.0, 0), 1, 0, LootStoreKind::Creature, 100,),
            generated_item(item(27, 0, 100.0, 0), 1, 1, LootStoreKind::Reference, 700,),
            generated_item(item(26, 0, 0.0, 1), 1, 2, LootStoreKind::Creature, 100,),
        ]
    );
}

#[test]
fn fill_personal_loot_assigns_plain_entries_to_one_looter_like_cpp() {
    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature
        .load_rows_like_cpp(
            [
                LootTemplateRow {
                    entry: 100,
                    item: item(25, 0, 100.0, 0),
                },
                LootTemplateRow {
                    entry: 100,
                    item: item(26, 0, 100.0, 0),
                },
            ],
            |item_id| matches!(item_id, 25 | 26),
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature);

    let looters = [guid(42), guid(77)];
    let mut rng = StdRng::seed_from_u64(7);
    let generated = stores[&LootStoreKind::Creature]
        .fill_personal_loot_with_context_like_cpp(
            100,
            LootStoreKind::Creature,
            &stores,
            LootFillOptions::default(),
            &looters,
            &mut rng,
            |_| Some(item_metadata(20)),
            |_| 1.0,
            |_, _| true,
            random_properties,
        )
        .unwrap();

    assert_eq!(generated.len(), 2);
    assert!(generated.iter().all(|item| looters.contains(&item.looter)));
    assert!(generated.iter().all(|item| item.item.loot_list_id < 2));
    assert_eq!(
        generated
            .iter()
            .map(|item| item.item.item_id)
            .collect::<Vec<_>>(),
        vec![25, 26]
    );
}

#[test]
fn fill_personal_loot_reference_cycles_looters_like_cpp() {
    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    let mut reference_item = item(0, 700, 100.0, 0);
    reference_item.max_count = 2;
    creature
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 100,
                item: reference_item,
            }],
            |_| true,
        )
        .unwrap();

    let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 700,
                item: item(27, 0, 100.0, 0),
            }],
            |item_id| item_id == 27,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature);
    stores.insert(LootStoreKind::Reference, reference);

    let looters = [guid(42), guid(77)];
    let mut rng = StdRng::seed_from_u64(3);
    let generated = stores[&LootStoreKind::Creature]
        .fill_personal_loot_with_context_like_cpp(
            100,
            LootStoreKind::Creature,
            &stores,
            LootFillOptions::default(),
            &looters,
            &mut rng,
            |_| Some(item_metadata(20)),
            |_| 1.0,
            |_, _| true,
            random_properties,
        )
        .unwrap();

    let mut assigned = generated.iter().map(|item| item.looter).collect::<Vec<_>>();
    assigned.sort_by_key(|guid| guid.counter());
    assert_eq!(assigned, looters);
    assert_eq!(
        generated
            .iter()
            .map(|item| item.item.item_id)
            .collect::<Vec<_>>(),
        vec![27, 27]
    );
    assert_eq!(
        generated
            .iter()
            .map(|item| item.item.loot_list_id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}
