//! Map store regressions.
//!
//! Moved out of map.rs under #683; every test is unchanged.

use super::*;

#[test]
fn map_store_flex_locking_flag_matches_cpp() {
    let store = MapStore::from_entries([MapEntry {
        id: 631,
        instance_type: 2,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: MAP_FLAG_FLEXIBLE_RAID_LOCKING,
        flags2: 0,
    }]);

    assert!(store.get(631).unwrap().is_flex_locking());
    assert!(store.get(1).is_none());
}

#[test]
fn map_store_ignore_instance_farm_limit_flag_matches_cpp() {
    let entry = MapEntry {
        id: 33,
        instance_type: MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT,
    };

    assert!(entry.ignores_instance_farm_limit_like_cpp());
}

#[test]
fn map_entry_classification_matches_cpp_helpers() {
    let world = MapEntry {
        id: 0,
        instance_type: MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };
    let dungeon = MapEntry {
        id: 33,
        instance_type: MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };
    let raid = MapEntry {
        id: 631,
        instance_type: MAP_RAID,
        expansion_id: 2,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };
    let battleground = MapEntry {
        id: 489,
        instance_type: MAP_BATTLEGROUND,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };
    let arena = MapEntry {
        id: 562,
        instance_type: MAP_ARENA,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };
    let garrison = MapEntry {
        id: 1152,
        instance_type: MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: MAP_FLAG_GARRISON,
        flags2: 0,
    };
    let split = MapEntry {
        id: 609,
        instance_type: MAP_COMMON,
        expansion_id: 0,
        parent_map_id: 571,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };

    assert!(world.is_world_map());
    assert!(dungeon.is_dungeon());
    assert!(raid.is_dungeon());
    assert_eq!(raid.expansion_like_cpp(), 2);
    assert!(battleground.is_battleground_or_arena());
    assert!(arena.is_battleground_or_arena());
    assert!(garrison.is_garrison());
    assert!(!garrison.is_dungeon());
    assert!(garrison.is_instanceable_like_cpp());
    assert!(!world.is_instanceable_like_cpp());
    assert!(split.is_split_by_faction());

    let pvp_item_level_map = MapEntry {
        id: 30_001,
        instance_type: MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP,
    };
    assert!(
        pvp_item_level_map.activates_pvp_item_levels_like_cpp(),
        "C++ Player::UpdateItemLevelAreaBasedScaling checks MapEntry::Flags[1] & 0x40"
    );
}

#[test]
fn map_store_parent_fields_match_cpp_load_info() {
    let store = MapStore::from_entries([
        MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 111,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: 1,
            flags1: 0,
            flags2: 0,
        },
    ]);

    let child = store.get(609).unwrap();
    assert_eq!(child.parent_map_id, 571);
    assert_eq!(child.cosmetic_parent_map_id, -1);
    let cosmetic = store.get(111).unwrap();
    assert_eq!(cosmetic.parent_map_id, -1);
    assert_eq!(cosmetic.cosmetic_parent_map_id, 1);
}

#[test]
fn terrain_root_map_id_follows_parent_chain_like_cpp() {
    let store = MapStore::from_entries([
        MapEntry {
            id: 1,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 571,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: 1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]);

    assert_eq!(store.terrain_root_map_id_like_cpp(609), Some(1));
    assert_eq!(store.terrain_root_map_id_like_cpp(571), Some(1));
    assert_eq!(store.terrain_root_map_id_like_cpp(1), Some(1));
}

#[test]
fn terrain_root_map_id_prefers_parent_over_cosmetic_like_cpp() {
    let store = MapStore::from_entries([
        MapEntry {
            id: 1,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 571,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: 1,
            flags1: 0,
            flags2: 0,
        },
    ]);

    assert_eq!(store.terrain_root_map_id_like_cpp(609), Some(571));
}

#[test]
fn terrain_root_map_id_handles_missing_entries_like_cpp() {
    let store = MapStore::from_entries([MapEntry {
        id: 609,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: 571,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]);

    assert_eq!(store.terrain_root_map_id_like_cpp(609), Some(609));
    assert_eq!(store.terrain_root_map_id_like_cpp(571), None);
}

#[test]
fn parent_child_map_data_matches_cpp_world_initialization() {
    let store = MapStore::from_entries([
        MapEntry {
            id: 1,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 571,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: 1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]);

    let map_data = store.parent_child_map_data_like_cpp();
    assert_eq!(
        map_data,
        vec![(1, vec![571]), (571, vec![609]), (609, Vec::new())]
    );
}

#[test]
fn parent_child_map_data_creates_missing_parent_bucket_like_cpp() {
    let store = MapStore::from_entries([MapEntry {
        id: 609,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: 571,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]);

    let map_data = store.parent_child_map_data_like_cpp();
    assert_eq!(map_data, vec![(571, vec![609]), (609, Vec::new())]);
}

#[test]
#[should_panic(expected = "inconsistent parent map data")]
fn parent_child_map_data_rejects_inconsistent_parent_like_cpp_assert() {
    let store = MapStore::from_entries([MapEntry {
        id: 609,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: 571,
        cosmetic_parent_map_id: 1,
        flags1: 0,
        flags2: 0,
    }]);

    let _ = store.parent_child_map_data_like_cpp();
}

#[test]
fn map_difficulty_store_indexes_by_map_and_difficulty_like_cpp() {
    let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
        id: 900,
        message: String::new(),
        map_id: 631,
        difficulty_id: 4,
        lock_id: 7,
        reset_interval: 2,
        max_players: 0,
        flags: MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
    }]);

    let entry = store.get(631, 4).unwrap();
    assert_eq!(entry.lock_id, 7);
    assert!(entry.is_using_encounter_locks());
    assert!(store.get(631, 3).is_none());
}

#[test]
fn default_for_map_prefers_default_difficulty_flag_like_cpp() {
    let difficulties = DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 3,
            instance_type: 2,
            flags: DifficultyFlags::CAN_SELECT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 15,
            instance_type: 2,
            flags: (DifficultyFlags::CAN_SELECT | DifficultyFlags::DEFAULT).bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);
    let store = MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 631,
            difficulty_id: 3,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        },
        MapDifficultyEntry {
            id: 901,
            message: String::new(),
            map_id: 631,
            difficulty_id: 15,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        },
    ]);

    let entry = store.default_for_map_like_cpp(631, &difficulties).unwrap();

    assert_eq!(entry.difficulty_id, 15);
}

#[test]
fn default_for_map_falls_back_to_any_map_difficulty_like_cpp() {
    let difficulties = DifficultyStore::from_entries([crate::DifficultyEntry {
        id: 3,
        instance_type: 2,
        flags: DifficultyFlags::CAN_SELECT.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }]);
    let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
        id: 900,
        message: String::new(),
        map_id: 631,
        difficulty_id: 3,
        lock_id: 0,
        reset_interval: 0,
        max_players: 0,
        flags: 0,
    }]);

    let entry = store.default_for_map_like_cpp(631, &difficulties).unwrap();

    assert_eq!(entry.difficulty_id, 3);
    assert!(store.default_for_map_like_cpp(632, &difficulties).is_none());
}

#[test]
fn downscaled_for_map_uses_exact_map_difficulty_like_cpp() {
    let difficulties = DifficultyStore::from_entries([crate::DifficultyEntry {
        id: 2,
        instance_type: 1,
        flags: 0,
        fallback_difficulty_id: 1,
        toggle_difficulty_id: 0,
    }]);
    let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
        id: 900,
        message: String::new(),
        map_id: 33,
        difficulty_id: 2,
        lock_id: 8,
        reset_interval: 1,
        max_players: 0,
        flags: 0,
    }]);

    let (entry, effective_difficulty) = store
        .downscaled_for_map_like_cpp(33, 2, &difficulties)
        .unwrap();

    assert_eq!(entry.lock_id, 8);
    assert_eq!(effective_difficulty, 2);
}

#[test]
fn downscaled_for_map_follows_fallback_difficulty_like_cpp() {
    let difficulties = DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 5,
            instance_type: 1,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 2,
            instance_type: 1,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
    ]);
    let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
        id: 900,
        message: String::new(),
        map_id: 33,
        difficulty_id: 2,
        lock_id: 8,
        reset_interval: 1,
        max_players: 0,
        flags: 0,
    }]);

    let (entry, effective_difficulty) = store
        .downscaled_for_map_like_cpp(33, 5, &difficulties)
        .unwrap();

    assert_eq!(entry.difficulty_id, 2);
    assert_eq!(entry.lock_id, 8);
    assert_eq!(effective_difficulty, 2);
}

#[test]
fn downscaled_for_map_falls_back_to_default_when_chain_breaks_like_cpp() {
    let difficulties = DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 5,
            instance_type: 1,
            flags: 0,
            fallback_difficulty_id: 99,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 1,
            instance_type: 1,
            flags: DifficultyFlags::DEFAULT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);
    let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
        id: 900,
        message: String::new(),
        map_id: 33,
        difficulty_id: 1,
        lock_id: 7,
        reset_interval: 0,
        max_players: 0,
        flags: 0,
    }]);

    let (entry, effective_difficulty) = store
        .downscaled_for_map_like_cpp(33, 5, &difficulties)
        .unwrap();

    assert_eq!(entry.difficulty_id, 1);
    assert_eq!(entry.lock_id, 7);
    assert_eq!(effective_difficulty, 1);
}

#[test]
fn map_difficulty_x_conditions_are_grouped_in_cpp_order() {
    let store = MapDifficultyXConditionStore::from_entries([
        MapDifficultyXConditionEntry {
            id: 10,
            failure_description: "late".to_string(),
            player_condition_id: 100,
            order_index: 20,
            map_difficulty_id: 7,
        },
        MapDifficultyXConditionEntry {
            id: 11,
            failure_description: "early".to_string(),
            player_condition_id: 101,
            order_index: 10,
            map_difficulty_id: 7,
        },
        MapDifficultyXConditionEntry {
            id: 12,
            failure_description: "other".to_string(),
            player_condition_id: 102,
            order_index: 1,
            map_difficulty_id: 8,
        },
    ]);

    let ids: Vec<_> = store
        .conditions_for_map_difficulty(7)
        .map(|entry| entry.id)
        .collect();
    assert_eq!(ids, vec![11, 10]);
}

#[test]
fn map_difficulty_x_condition_failure_matches_cpp_first_unmet_existing_condition() {
    let store = MapDifficultyXConditionStore::from_entries([
        MapDifficultyXConditionEntry {
            id: 10,
            failure_description: String::new(),
            player_condition_id: 100,
            order_index: 10,
            map_difficulty_id: 7,
        },
        MapDifficultyXConditionEntry {
            id: 11,
            failure_description: String::new(),
            player_condition_id: 999,
            order_index: 20,
            map_difficulty_id: 7,
        },
        MapDifficultyXConditionEntry {
            id: 12,
            failure_description: String::new(),
            player_condition_id: 101,
            order_index: 30,
            map_difficulty_id: 7,
        },
    ]);
    let player_conditions = PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 100,
            ..Default::default()
        },
        PlayerConditionEntry {
            id: 101,
            ..Default::default()
        },
    ]);

    assert_eq!(
        store.failed_condition_like_cpp(7, &player_conditions, |condition| condition.id == 100),
        Some(12)
    );
}

#[test]
fn load_map_and_map_difficulty_db2_when_fixtures_exist() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.join("Map.db2").exists() || !dbc_dir.join("MapDifficulty.db2").exists() {
        eprintln!("Skipping test: Map.db2/MapDifficulty.db2 not found");
        return;
    }

    let maps = MapStore::load(data_dir, locale).expect("failed to load maps");
    let difficulties =
        MapDifficultyStore::load(data_dir, locale).expect("failed to load map difficulties");
    let difficulty_conditions = if dbc_dir.join("MapDifficultyXCondition.db2").exists() {
        Some(
            MapDifficultyXConditionStore::load(data_dir, locale)
                .expect("failed to load map difficulty conditions"),
        )
    } else {
        None
    };

    assert!(!maps.is_empty());
    assert!(!difficulties.is_empty());
    if let Some(difficulty_conditions) = difficulty_conditions {
        assert!(
            difficulty_conditions
                .by_id
                .values()
                .all(|condition| condition.map_difficulty_id != 0)
        );
    }
    assert!(maps.get(0).is_some());

    let icc = maps.get(631).expect("Icecrown Citadel map missing");
    assert_eq!(icc.instance_type, 2);

    let known_difficulty = difficulties
        .get(32, 4)
        .expect("known MapDifficulty row for map 32 difficulty 4 missing");
    assert_eq!(known_difficulty.map_id, 32);
    assert_eq!(known_difficulty.difficulty_id, 4);
    assert_eq!(known_difficulty.reset_interval, 2);

    let northrend_normal = difficulties
        .get(571, 0)
        .expect("known MapDifficulty row for Northrend map 571 difficulty 0 missing");
    assert_eq!(northrend_normal.map_id, 571);
    assert_eq!(northrend_normal.difficulty_id, 0);
}
