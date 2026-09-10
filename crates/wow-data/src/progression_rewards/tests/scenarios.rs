//! Progression-reward store regressions.
//!
//! Moved out of progression_rewards.rs under #685; every test is unchanged.

use super::*;

#[test]
fn scaling_stat_values_helpers_match_cpp_mask_priority() {
    let values = scaling_stat_values_for_test(1, 80);

    assert_eq!(values.ssd_multiplier_like_cpp(0), 0);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00000001), 201);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00000002), 202);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00000004), 203);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00000008), 204);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00000010), 205);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00040000), 206);
    assert_eq!(values.ssd_multiplier_like_cpp(0x00040001), 201);

    assert_eq!(values.armor_mod_like_cpp(0), 0);
    assert_eq!(values.armor_mod_like_cpp(0x00000020), 301);
    assert_eq!(values.armor_mod_like_cpp(0x00000040), 302);
    assert_eq!(values.armor_mod_like_cpp(0x00000080), 303);
    assert_eq!(values.armor_mod_like_cpp(0x00000100), 304);
    assert_eq!(values.armor_mod_like_cpp(0x00080000), 0);
    assert_eq!(values.armor_mod_like_cpp(0x00180000), 305);
    assert_eq!(values.armor_mod_like_cpp(0x00100000), 306);
    assert_eq!(values.armor_mod_like_cpp(0x00200000), 307);
    assert_eq!(values.armor_mod_like_cpp(0x00400000), 308);
    assert_eq!(values.armor_mod_like_cpp(0x00800000), 309);
    assert_eq!(values.armor_mod_like_cpp(0x00800020), 301);

    assert_eq!(values.dps_mod_like_cpp(0), 0);
    assert_eq!(values.dps_mod_like_cpp(0x00000200), 101);
    assert_eq!(values.dps_mod_like_cpp(0x00000400), 102);
    assert_eq!(values.dps_mod_like_cpp(0x00000800), 103);
    assert_eq!(values.dps_mod_like_cpp(0x00001000), 104);
    assert_eq!(values.dps_mod_like_cpp(0x00002000), 105);
    assert_eq!(values.dps_mod_like_cpp(0x00004000), 106);
    assert_eq!(values.dps_mod_like_cpp(0x00004200), 101);

    assert!(!values.is_two_hand_like_cpp(0x00000200));
    assert!(values.is_two_hand_like_cpp(0x00000400));
    assert!(values.is_two_hand_like_cpp(0x00001000));
    assert_eq!(values.spell_bonus_like_cpp(0), 0);
    assert_eq!(values.spell_bonus_like_cpp(0x00008000), 107);
}

#[test]
fn scaling_stat_values_store_gets_entry_by_character_level_like_cpp() {
    let store = ScalingStatValuesStore::from_entries([
        scaling_stat_values_for_test(10, 10),
        scaling_stat_values_for_test(80, 80),
    ]);

    assert_eq!(
        store
            .get_for_character_level_like_cpp(80)
            .map(|entry| entry.id),
        Some(80)
    );
    assert_eq!(store.get_for_character_level_like_cpp(11), None);
}

#[test]
fn reward_pack_currency_store_uses_cpp_parent_relationship() {
    let store = RewardPackXCurrencyTypeStore::from_entries([RewardPackXCurrencyTypeEntry {
        id: 1,
        currency_type_id: 2,
        quantity: 3,
        reward_pack_id: 4,
    }]);

    assert_eq!(store.get(1).unwrap().reward_pack_id, 4);
}

#[test]
fn faction_template_friendly_relation_matches_cpp_precedence() {
    let mut faction = faction_template_for_test(1, 100, 0x02, 0x04, 0);
    let other = faction_template_for_test(2, 200, 0x04, 0, 0);

    assert!(faction.is_friendly_to_like_cpp(&faction));
    assert!(faction.is_friendly_to_like_cpp(&other));

    faction.enemies[0] = 200;
    faction.friend[0] = 200;
    assert!(!faction.is_friendly_to_like_cpp(&other));
}

#[test]
fn faction_template_hostile_relation_matches_cpp_precedence() {
    let mut faction = faction_template_for_test(1, 100, 0, 0, 0x04);
    let other = faction_template_for_test(2, 200, 0x04, 0, 0);

    assert!(!faction.is_hostile_to_like_cpp(&faction));
    assert!(faction.is_hostile_to_like_cpp(&other));

    faction.friend[0] = 200;
    assert!(!faction.is_hostile_to_like_cpp(&other));
}

#[test]
fn faction_template_misc_helpers_match_cpp_flags_and_groups() {
    let mut faction = faction_template_for_test(1, 100, 0, 0, FACTION_MASK_PLAYER_LIKE_CPP);
    faction.flags = FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP
        | FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT_LIKE_CPP;

    assert!(faction.is_hostile_to_players_like_cpp());
    assert!(faction.is_contested_guard_faction_like_cpp());
    assert!(faction.is_hostile_by_default_like_cpp());
    assert!(!faction.is_neutral_to_all_like_cpp());

    faction.enemy_group = 0;
    faction.flags = 0;
    assert!(faction.is_neutral_to_all_like_cpp());

    faction.enemies[7] = 200;
    assert!(!faction.is_neutral_to_all_like_cpp());
}

#[test]
fn quest_v2_unique_bit_flag_missing_quest_returns_zero_like_cpp() {
    let store = QuestV2Store::from_entries([]);

    assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 0);
}

#[test]
fn quest_v2_unique_bit_flag_existing_nonzero_returns_exact_flag_like_cpp() {
    let store = QuestV2Store::from_entries([QuestV2Entry {
        id: 12_345,
        unique_bit_flag: 77,
    }]);

    assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 77);
}

#[test]
fn quest_v2_unique_bit_flag_existing_zero_stays_zero_like_cpp() {
    let store = QuestV2Store::from_entries([QuestV2Entry {
        id: 12_345,
        unique_bit_flag: 0,
    }]);

    assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 0);
}

#[test]
fn quest_v2_unique_bit_flag_duplicate_ids_preserve_from_entries_last_wins() {
    let store = QuestV2Store::from_entries([
        QuestV2Entry {
            id: 12_345,
            unique_bit_flag: 11,
        },
        QuestV2Entry {
            id: 12_345,
            unique_bit_flag: 99,
        },
    ]);

    assert_eq!(store.len(), 1);
    assert_eq!(store.get_quest_unique_bit_flag_like_cpp(12_345), 99);
}

#[test]
fn quest_package_items_split_primary_and_fallback_like_cpp() {
    let store = QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: 10,
            item_id: 100,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP,
        },
        QuestPackageItemEntry {
            id: 2,
            package_id: 10,
            item_id: 200,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        },
        QuestPackageItemEntry {
            id: 3,
            package_id: 11,
            item_id: 300,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ]);

    let primary: Vec<i32> = store
        .quest_package_items_like_cpp(10)
        .map(|entry| entry.item_id)
        .collect();
    let fallback: Vec<i32> = store
        .quest_package_items_fallback_like_cpp(10)
        .map(|entry| entry.item_id)
        .collect();

    assert_eq!(primary, vec![100]);
    assert_eq!(fallback, vec![200]);
}

#[test]
fn num_talents_at_level_matches_cpp_class_and_fallback_like_cpp() {
    let store = NumTalentsAtLevelStore::from_entries([
        NumTalentsAtLevelEntry {
            id: 10,
            num_talents: 1,
            num_talents_death_knight: 0,
            num_talents_demon_hunter: 3,
        },
        NumTalentsAtLevelEntry {
            id: 80,
            num_talents: 71,
            num_talents_death_knight: 76,
            num_talents_demon_hunter: 0,
        },
    ]);

    assert_eq!(store.num_talents_at_level_like_cpp(10, 1), 1);
    assert_eq!(store.num_talents_at_level_like_cpp(80, 6), 76);
    assert_eq!(store.num_talents_at_level_like_cpp(90, 1), 71);
    assert_eq!(store.num_talents_at_level_like_cpp(90, 12), 0);
}

#[test]
fn curve_points_group_sort_and_skip_missing_curves_like_cpp() {
    let curves = CurveStore::from_entries([curve(10, 0)]);
    let points = CurvePointStore::from_entries([
        point(1, 10, 2, 20.0, 200.0),
        point(2, 999, 0, 1.0, 1.0),
        point(3, 10, 0, 0.0, 100.0),
        point(4, 10, 1, 10.0, 150.0),
    ]);

    let grouped = points.points_by_curve_like_cpp(&curves);
    assert_eq!(
        grouped.get(&10).unwrap(),
        &vec![[0.0, 100.0], [10.0, 150.0], [20.0, 200.0]]
    );
    assert!(!grouped.contains_key(&999));
}

#[test]
fn curve_x_axis_range_uses_ordered_curve_points_like_cpp() {
    let curves = CurveStore::from_entries([curve(10, 0)]);
    let points = CurvePointStore::from_entries([
        point(1, 10, 2, 20.0, 200.0),
        point(2, 999, 0, 1.0, 1.0),
        point(3, 10, 0, 0.0, 100.0),
        point(4, 10, 1, 10.0, 150.0),
    ]);

    assert_eq!(
        curves.curve_x_axis_range_like_cpp(&points, 10),
        Some((0.0, 20.0))
    );
    assert_eq!(curves.curve_x_axis_range_like_cpp(&points, 999), None);
}

#[test]
fn curve_value_at_linear_and_constant_match_cpp() {
    let curves = CurveStore::from_entries([curve(1, 0), curve(2, 0)]);
    let points = CurvePointStore::from_entries([
        point(1, 1, 0, 0.0, 10.0),
        point(2, 1, 1, 10.0, 30.0),
        point(3, 2, 0, 0.0, 77.0),
    ]);

    assert_close(curves.curve_value_at_like_cpp(&points, 1, -1.0), 10.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 1, 5.0), 20.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 1, 11.0), 30.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 2, 999.0), 77.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 999, 5.0), 0.0);
}

#[test]
fn curve_value_at_cosine_matches_cpp() {
    let curves = CurveStore::from_entries([curve(10, 3)]);
    let points =
        CurvePointStore::from_entries([point(1, 10, 0, 0.0, 10.0), point(2, 10, 1, 10.0, 30.0)]);

    assert_close(curves.curve_value_at_like_cpp(&points, 10, 5.0), 20.0);
}

#[test]
fn curve_value_at_catmull_rom_matches_cpp() {
    let curves = CurveStore::from_entries([curve(20, 1)]);
    let points = CurvePointStore::from_entries([
        point(1, 20, 0, 0.0, 0.0),
        point(2, 20, 1, 10.0, 10.0),
        point(3, 20, 2, 20.0, 20.0),
        point(4, 20, 3, 30.0, 30.0),
    ]);

    assert_close(curves.curve_value_at_like_cpp(&points, 20, 5.0), 10.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 20, 15.0), 15.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 20, 25.0), 20.0);
}

#[test]
fn curve_value_at_bezier_modes_match_cpp() {
    let curves = CurveStore::from_entries([curve(30, 2), curve(31, 2), curve(32, 2), curve(33, 2)]);
    let points = CurvePointStore::from_entries([
        point(1, 30, 0, 0.0, 10.0),
        point(2, 30, 1, 10.0, 30.0),
        point(3, 30, 2, 20.0, 10.0),
        point(4, 31, 0, 0.0, 0.0),
        point(5, 31, 1, 10.0, 30.0),
        point(6, 31, 2, 20.0, 30.0),
        point(7, 31, 3, 30.0, 0.0),
        point(8, 32, 0, 0.0, 0.0),
        point(9, 32, 1, 10.0, 10.0),
        point(10, 32, 2, 20.0, 20.0),
        point(11, 32, 3, 30.0, 30.0),
        point(12, 32, 4, 40.0, 40.0),
        point(13, 33, 0, 0.0, 44.0),
    ]);

    assert_close(curves.curve_value_at_like_cpp(&points, 30, 10.0), 20.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 31, 15.0), 22.5);
    assert_close(curves.curve_value_at_like_cpp(&points, 32, 20.0), 20.0);
    assert_close(curves.curve_value_at_like_cpp(&points, 33, 999.0), 44.0);
}

#[test]
fn content_tuning_data_filters_for_item_like_cpp() {
    let store = ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 100,
            min_level: 10,
            max_level: 70,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
        ContentTuningEntry {
            id: 101,
            min_level: 20,
            max_level: 60,
            flags: CONTENT_TUNING_FLAG_DISABLED_FOR_ITEM_LIKE_CPP,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ]);

    assert_eq!(
        store.content_tuning_data_like_cpp(100, true),
        Some(ContentTuningLevelsLikeCpp {
            min_level: 10,
            max_level: 70,
            min_level_with_delta: 10,
            max_level_with_delta: 70,
            target_level_min: 10,
            target_level_max: 70,
        })
    );
    assert_eq!(
        store
            .content_tuning_data_like_cpp(101, false)
            .unwrap()
            .max_level,
        60
    );
    assert_eq!(store.content_tuning_data_like_cpp(101, true), None);
    assert_eq!(store.content_tuning_data_like_cpp(999, true), None);
}

#[test]
fn content_tuning_data_clamps_levels_like_cpp() {
    let store = ContentTuningStore::from_entries([ContentTuningEntry {
        id: 200,
        min_level: -10,
        max_level: i32::from(MAX_LEVEL_LIKE_CPP) + 20,
        flags: 0,
        expected_stat_mod_id: 0,
        difficulty_esm_id: 0,
    }]);

    assert_eq!(
        store.content_tuning_data_like_cpp(200, true),
        Some(ContentTuningLevelsLikeCpp {
            min_level: 1,
            max_level: i32::from(MAX_LEVEL_LIKE_CPP),
            min_level_with_delta: 1,
            max_level_with_delta: i32::from(MAX_LEVEL_LIKE_CPP),
            target_level_min: 1,
            target_level_max: i32::from(MAX_LEVEL_LIKE_CPP),
        })
    );
}

#[test]
fn load_progression_rewards_db2_subbatch_when_fixtures_exist() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.exists() {
        eprintln!(
            "Skipping test: DB2 fixture directory not found at {}",
            dbc_dir.display()
        );
        return;
    }

    macro_rules! load_if_exists {
        ($file:literal, $store:ty) => {
            if dbc_dir.join($file).exists() {
                let _store = <$store>::load(data_dir, locale)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
            }
        };
    }

    load_if_exists!("Achievement_Category.db2", AchievementCategoryStore);
    load_if_exists!("ContentTuning.db2", ContentTuningStore);
    load_if_exists!("CriteriaTree.db2", CriteriaTreeStore);
    load_if_exists!("Curve.db2", CurveStore);
    load_if_exists!("CurvePoint.db2", CurvePointStore);
    load_if_exists!("Faction.db2", FactionStore);
    load_if_exists!("FactionTemplate.db2", FactionTemplateStore);
    load_if_exists!("FriendshipRepReaction.db2", FriendshipRepReactionStore);
    load_if_exists!("FriendshipReputation.db2", FriendshipReputationStore);
    load_if_exists!("ModifierTree.db2", ModifierTreeStore);
    load_if_exists!("NumTalentsAtLevel.db2", NumTalentsAtLevelStore);
    load_if_exists!("ParagonReputation.db2", ParagonReputationStore);
    load_if_exists!("QuestFactionReward.db2", QuestFactionRewardStore);
    load_if_exists!("QuestInfo.db2", QuestInfoStore);
    load_if_exists!("QuestLineXQuest.db2", QuestLineXQuestStore);
    load_if_exists!("QuestMoneyReward.db2", QuestMoneyRewardStore);
    load_if_exists!("QuestPackageItem.db2", QuestPackageItemStore);
    load_if_exists!("QuestSort.db2", QuestSortStore);
    load_if_exists!("QuestV2.db2", QuestV2Store);
    load_if_exists!("RewardPack.db2", RewardPackStore);
    load_if_exists!("RewardPackXCurrencyType.db2", RewardPackXCurrencyTypeStore);
    load_if_exists!("RewardPackXItem.db2", RewardPackXItemStore);
    load_if_exists!("ScalingStatDistribution.db2", ScalingStatDistributionStore);
    load_if_exists!("ScalingStatValues.db2", ScalingStatValuesStore);
}

#[test]
fn quest_money_reward_level_80_pallet_array_matches_cpp_fixture() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.join("QuestMoneyReward.db2").exists() {
        return;
    }

    let store = QuestMoneyRewardStore::load(data_dir, locale).expect("load QuestMoneyReward.db2");
    let row = store.get(80).expect("level 80 QuestMoneyReward row");
    assert_eq!(row.difficulty[2], 19_000);
    assert_eq!(row.difficulty[5], 74_000);
    assert_eq!(row.difficulty[7], 222_000);
    assert_eq!(row.difficulty[8], 296_000);
}
