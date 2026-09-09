//! Loot store definitions and templates regression scenarios, part 2 of 2.
//!
//! Moved out of the lib.rs root under #642; every test is unchanged.

use super::*;

#[test]
fn fill_loot_with_context_reports_reference_source_like_cpp_conditions() {
    let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 900,
                item: item(27, 0, 100.0, 0),
            }],
            |item_id| item_id == 27,
        )
        .unwrap();

    let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 100,
                item: LootStoreItem {
                    reference: 900,
                    max_count: 1,
                    ..item(900, 900, 100.0, 0)
                },
            }],
            |_| false,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature);
    stores.insert(LootStoreKind::Reference, reference);

    let mut seen = Vec::new();
    let mut rng = StdRng::seed_from_u64(7);
    let generated = stores[&LootStoreKind::Creature]
        .fill_loot_with_context_like_cpp(
            100,
            LootStoreKind::Creature,
            &stores,
            LootFillOptions::default(),
            &mut rng,
            |_| Some(item_metadata(20)),
            |_| 1.0,
            |context| {
                seen.push((context.store_kind, context.entry, context.item.item_id));
                true
            },
            random_properties,
        )
        .unwrap();

    assert_eq!(
        generated,
        vec![generated_item(
            item(27, 0, 100.0, 0),
            1,
            0,
            LootStoreKind::Reference,
            900
        )]
    );
    assert_eq!(seen, vec![(LootStoreKind::Reference, 900, 27)]);
    assert_eq!(
        stores[&LootStoreKind::Creature].condition_ids_for_fill_like_cpp(
            100,
            LootStoreKind::Creature,
            &stores,
        ),
        vec![LootConditionId {
            source_type: 10,
            source_group: 900,
            source_entry: 27,
        }]
    );
}

#[test]
fn fill_loot_splits_stacks_and_caps_at_cpp_max_loot_items() {
    let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
    store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 500,
                item: LootStoreItem {
                    min_count: 40,
                    max_count: 40,
                    ..item(25, 0, 100.0, 0)
                },
            }],
            |item_id| item_id == 25,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Item, store);

    let mut rng = StdRng::seed_from_u64(11);
    let generated = stores[&LootStoreKind::Item]
        .fill_loot_like_cpp(
            500,
            LootStoreKind::Item,
            &stores,
            LootFillOptions::default(),
            &mut rng,
            |_| Some(item_metadata(1)),
            |_| 1.0,
            |_| true,
            random_properties,
        )
        .unwrap();

    assert_eq!(generated.len(), 18);
    assert_eq!(generated[0].loot_list_id, 0);
    assert_eq!(generated[17].loot_list_id, 17);
    assert!(
        generated
            .iter()
            .all(|item| item.item_id == 25 && item.count == 1)
    );
}

#[test]
fn fill_loot_initializes_loot_item_metadata_like_cpp_constructor() {
    let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
    store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 600,
                item: LootStoreItem {
                    needs_quest: true,
                    ..item(25, 0, 100.0, 0)
                },
            }],
            |item_id| item_id == 25,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Item, store);

    let mut rng = StdRng::seed_from_u64(13);
    let generated = stores[&LootStoreKind::Item]
        .fill_loot_like_cpp(
            600,
            LootStoreKind::Item,
            &stores,
            LootFillOptions {
                item_context: 4,
                ..LootFillOptions::default()
            },
            &mut rng,
            |_| {
                Some(LootItemTemplateMetadata {
                    max_stack: 20,
                    has_multi_drop_flag: true,
                    has_follow_loot_rules_flag: true,
                })
            },
            |_| 1.0,
            |_| true,
            |_, _| LootItemRandomProperties {
                id: 4242,
                seed: 2424,
            },
        )
        .unwrap();

    assert_eq!(
        generated,
        vec![GeneratedLootItem {
            item_id: 25,
            count: 1,
            loot_list_id: 0,
            random_properties_id: 4242,
            random_properties_seed: 2424,
            context: 4,
            store_item_context: LootStoreItemContext {
                store_kind: LootStoreKind::Item,
                entry: 600,
                item: LootStoreItem {
                    needs_quest: true,
                    ..item(25, 0, 100.0, 0)
                },
            },
            free_for_all: true,
            follow_loot_rules: true,
            needs_quest: true,
            is_looted: false,
            is_blocked: false,
            is_under_threshold: false,
            is_counted: false,
        }]
    );
}

#[test]
fn fill_loot_random_properties_callback_uses_fill_rng_like_cpp() {
    let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
    store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 601,
                item: item(25, 0, 100.0, 0),
            }],
            |item_id| item_id == 25,
        )
        .unwrap();

    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Item, store);

    let mut expected_rng = StdRng::seed_from_u64(14);
    let _: u32 = expected_rng.gen_range(1..=1);
    let expected_id = expected_rng.gen_range(1..=10_000);

    let mut rng = StdRng::seed_from_u64(14);
    let generated = stores[&LootStoreKind::Item]
        .fill_loot_like_cpp(
            601,
            LootStoreKind::Item,
            &stores,
            LootFillOptions::default(),
            &mut rng,
            |_| Some(item_metadata(20)),
            |_| 1.0,
            |_| true,
            |_, rng| LootItemRandomProperties {
                id: rng.gen_range(1..=10_000),
                seed: 0,
            },
        )
        .unwrap();

    assert_eq!(generated[0].random_properties_id, expected_id);
}

#[test]
fn fill_loot_reports_missing_template_like_cpp_no_empty_error_branch() {
    let store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, store);

    let mut rng = StdRng::seed_from_u64(1);
    let err = stores[&LootStoreKind::Creature]
        .fill_loot_like_cpp(
            999,
            LootStoreKind::Creature,
            &stores,
            LootFillOptions::default(),
            &mut rng,
            |_| Some(item_metadata(1)),
            |_| 1.0,
            |_| true,
            random_properties,
        )
        .unwrap_err();

    assert_eq!(err, LootFillError::MissingLootTemplate { loot_id: 999 });
}

#[test]
fn generate_money_loot_matches_cpp_boundary_branches() {
    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    assert_eq!(
        generate_money_loot_with_rate_like_cpp(0, 0, 1.0, &mut rng),
        0
    );
    assert_eq!(
        generate_money_loot_with_rate_like_cpp(120, 100, 1.0, &mut rng),
        100
    );
    assert_eq!(
        generate_money_loot_with_rate_like_cpp(10, 10, 1.0, &mut rng),
        10
    );
    assert_eq!(
        generate_money_loot_with_rate_like_cpp(120, 100, 2.5, &mut rng),
        250
    );

    let small_range = generate_money_loot_with_rate_like_cpp(100, 200, 1.0, &mut rng);
    assert!((100..=200).contains(&small_range));

    let wide_range = generate_money_loot_with_rate_like_cpp(1_000, 100_000, 1.0, &mut rng);
    assert_eq!(wide_range & 0xFF, 0);
    assert!((((1_000 >> 8) << 8)..=((100_000 >> 8) << 8)).contains(&wide_range));
}

#[test]
fn loot_item_ui_type_hides_looted_or_disallowed_rows_like_cpp() {
    let player = guid(42);
    let other = guid(77);

    assert_eq!(
        ui_type(
            player,
            &[player],
            true,
            false,
            false,
            false,
            true,
            LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        None
    );
    assert_eq!(
        ui_type(
            player,
            &[other],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        None
    );
}

#[test]
fn loot_item_ui_type_free_for_all_and_quest_paths_match_cpp() {
    let player = guid(42);

    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            true,
            true,
            false,
            true,
            LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            true,
            true,
            false,
            true,
            LOOT_METHOD_GROUP_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            true,
            false,
            false,
            true,
            LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        None
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            true,
            false,
            LOOT_METHOD_GROUP_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
    );
}

#[test]
fn loot_item_ui_type_round_robin_and_master_loot_match_cpp() {
    let player = guid(42);
    let other = guid(77);

    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
            other,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        None
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_MASTER_LIKE_CPP,
            ObjectGuid::EMPTY,
            player,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_MASTER_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_MASTER_LIKE_CPP,
            ObjectGuid::EMPTY,
            other,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_LOCKED_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_MASTER_LIKE_CPP,
            ObjectGuid::EMPTY,
            other,
            true,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
    );
}

#[test]
fn loot_item_ui_type_group_roll_paths_match_cpp() {
    let player = guid(42);
    let other = guid(77);

    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_GROUP_LIKE_CPP,
            other,
            ObjectGuid::EMPTY,
            true,
            false,
            ObjectGuid::EMPTY,
        ),
        None
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            true,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_GROUP_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_GROUP_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            player,
        ),
        Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP)
    );
    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_GROUP_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            other,
        ),
        None
    );
}

#[test]
fn loot_item_ui_type_personal_loot_matches_cpp() {
    let player = guid(42);

    assert_eq!(
        ui_type(
            player,
            &[player],
            false,
            false,
            false,
            false,
            true,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            false,
            false,
            ObjectGuid::EMPTY,
        ),
        Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP)
    );
}

#[test]
fn have_quest_loot_follows_reference_templates_like_cpp() {
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 700,
                item: item(701, 701, 100.0, 0),
            }],
            |_| true,
        )
        .unwrap();
    let mut quest_item = item(9001, 0, 100.0, 0);
    quest_item.needs_quest = true;
    let mut reference_store = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 701,
                item: quest_item,
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    stores.insert(LootStoreKind::Reference, reference_store);
    let store = stores.get(&LootStoreKind::Gameobject).unwrap();

    assert!(store.have_quest_loot_for_like_cpp(700, &stores));
    assert!(store.have_quest_loot_for_player_like_cpp(700, &stores, |item_id| item_id == 9001));
    assert!(!store.have_quest_loot_for_player_like_cpp(700, &stores, |item_id| item_id == 42));
}
