//! Item collection regressions.
//!
//! Moved out of item_collections.rs under #685; every test is unchanged.

use super::*;

#[test]
fn battle_pet_species_flags_match_cpp_masks() {
    let entry = BattlePetSpeciesEntry {
        id: 11,
        description: String::new(),
        source_text: String::new(),
        creature_id: 0,
        summon_spell_id: 0,
        icon_file_data_id: 0,
        pet_type_enum: 0,
        flags: BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP
            | BATTLE_PET_SPECIES_FLAG_NOT_TRADABLE_LIKE_CPP,
        source_type_enum: 0,
        card_ui_model_scene_id: 0,
        loadout_ui_model_scene_id: 0,
    };

    assert!(entry.cant_battle_like_cpp());
    assert!(entry.not_tradable_like_cpp());
    assert!(!entry.has_flag_like_cpp(0x00020));
}

#[test]
fn calculate_battle_pet_stats_matches_cpp_formula() {
    let breed_states = BattlePetBreedStateStore::from_entries([
        BattlePetBreedStateEntry {
            id: 1,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
            value: 500,
            battle_pet_breed_id: 7,
        },
        BattlePetBreedStateEntry {
            id: 2,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
            value: 300,
            battle_pet_breed_id: 7,
        },
        BattlePetBreedStateEntry {
            id: 3,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
            value: 200,
            battle_pet_breed_id: 7,
        },
    ]);
    let species_states = BattlePetSpeciesStateStore::from_entries([
        BattlePetSpeciesStateEntry {
            id: 10,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
            value: 100,
            battle_pet_species_id: 11,
        },
        BattlePetSpeciesStateEntry {
            id: 11,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
            value: 50,
            battle_pet_species_id: 11,
        },
        BattlePetSpeciesStateEntry {
            id: 12,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
            value: 25,
            battle_pet_species_id: 11,
        },
    ]);
    let qualities = BattlePetBreedQualityStore::from_entries([BattlePetBreedQualityEntry {
        id: 20,
        state_multiplier: 1.5,
        quality_enum: 3,
    }]);

    assert_eq!(
        calculate_battle_pet_stats_like_cpp(
            7,
            11,
            3,
            2,
            &breed_states,
            &species_states,
            &qualities,
        ),
        Some(BattlePetCalculatedStatsLikeCpp {
            max_health: 190,
            power: 11,
            speed: 7,
        })
    );
}

#[test]
fn calculate_battle_pet_stats_requires_existing_breed_but_defaults_missing_states_like_cpp() {
    let breed_states = BattlePetBreedStateStore::from_entries([BattlePetBreedStateEntry {
        id: 1,
        battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
        value: 250,
        battle_pet_breed_id: 7,
    }]);
    let species_states = BattlePetSpeciesStateStore::from_entries([BattlePetSpeciesStateEntry {
        id: 10,
        battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
        value: 80,
        battle_pet_species_id: 11,
    }]);
    let qualities = BattlePetBreedQualityStore::from_entries([]);

    assert_eq!(
        calculate_battle_pet_stats_like_cpp(
            7,
            11,
            9,
            3,
            &breed_states,
            &species_states,
            &qualities,
        ),
        Some(BattlePetCalculatedStatsLikeCpp {
            max_health: 112,
            power: 8,
            speed: 0,
        })
    );
    assert_eq!(
        calculate_battle_pet_stats_like_cpp(
            8,
            11,
            9,
            3,
            &breed_states,
            &species_states,
            &qualities,
        ),
        None
    );
}

#[test]
fn heirloom_store_preserves_cpp_upgrade_arrays() {
    let store = HeirloomStore::from_entries([HeirloomEntry {
        id: 42,
        source_text: "vendor".to_string(),
        item_id: 100,
        legacy_upgraded_item_id: 101,
        static_upgraded_item_id: 102,
        source_type_enum: 3,
        flags: 4,
        legacy_item_id: 99,
        upgrade_item_id: [1, 2, 3, 4, 5, 6],
        upgrade_item_bonus_list_id: [11, 12, 13, 14, 15, 16],
    }]);

    assert_eq!(store.get(42).unwrap().upgrade_item_id[5], 6);
    assert_eq!(store.get(42).unwrap().upgrade_item_bonus_list_id[0], 11);
    assert_eq!(store.get_by_item_id_like_cpp(100).unwrap().id, 42);
    assert!(store.get_by_item_id_like_cpp(404).is_none());
}

#[test]
fn toy_store_indexes_by_item_id_like_cpp() {
    let store = ToyStore::from_entries([ToyEntry {
        id: 7,
        source_text: "drop".to_string(),
        item_id: 30_000,
        flags: 2,
        source_type_enum: 4,
    }]);

    assert_eq!(store.get_by_item_id_like_cpp(30_000).unwrap().id, 7);
    assert!(store.get_by_item_id_like_cpp(40_000).is_none());
}

#[test]
fn transmog_set_items_keep_cpp_secondary_index_shape() {
    let store = TransmogSetItemStore::from_entries_and_sets(
        [
            TransmogSetItemEntry {
                id: 10,
                transmog_set_id: 7,
                item_modified_appearance_id: 100,
                flags: 0,
            },
            TransmogSetItemEntry {
                id: 11,
                transmog_set_id: 8,
                item_modified_appearance_id: 200,
                flags: 1,
            },
            TransmogSetItemEntry {
                id: 12,
                transmog_set_id: 7,
                item_modified_appearance_id: 101,
                flags: 2,
            },
            TransmogSetItemEntry {
                id: 13,
                transmog_set_id: 99,
                item_modified_appearance_id: 100,
                flags: 3,
            },
        ],
        [
            TransmogSetEntry {
                id: 7,
                name: "set 7".to_string(),
                class_mask: 0,
                tracking_quest_id: 0,
                flags: 0,
                transmog_set_group_id: 70,
                item_name_description_id: 0,
                parent_transmog_set_id: 0,
                expansion_id: 0,
                ui_order: 0,
            },
            TransmogSetEntry {
                id: 8,
                name: "set 8".to_string(),
                class_mask: 0,
                tracking_quest_id: 0,
                flags: 0,
                transmog_set_group_id: 80,
                item_name_description_id: 0,
                parent_transmog_set_id: 0,
                expansion_id: 0,
                ui_order: 0,
            },
        ],
    );

    assert_eq!(store.get(11).unwrap().item_modified_appearance_id, 200);
    assert_eq!(
        store
            .get_transmog_set_items_like_cpp(7)
            .unwrap()
            .iter()
            .map(|entry| entry.item_modified_appearance_id)
            .collect::<Vec<_>>(),
        vec![100, 101]
    );
    assert!(store.get_transmog_set_items_like_cpp(99).is_none());
    assert_eq!(
        store
            .get_transmog_sets_for_item_modified_appearance_like_cpp(100)
            .unwrap()
            .iter()
            .map(|set| (set.id, set.transmog_set_group_id))
            .collect::<Vec<_>>(),
        vec![(7, 70)]
    );
    assert!(
        store
            .get_transmog_sets_for_item_modified_appearance_like_cpp(999)
            .is_none()
    );
}

#[test]
fn load_item_collections_db2_subbatch_when_fixtures_exist() {
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

    load_if_exists!("AuctionHouse.db2", AuctionHouseStore);
    load_if_exists!("BankBagSlotPrices.db2", BankBagSlotPricesStore);
    load_if_exists!("BattlePetAbility.db2", BattlePetAbilityStore);
    load_if_exists!("BattlePetBreedQuality.db2", BattlePetBreedQualityStore);
    load_if_exists!("BattlePetBreedState.db2", BattlePetBreedStateStore);
    load_if_exists!("BattlePetSpeciesState.db2", BattlePetSpeciesStateStore);
    load_if_exists!("CurrencyContainer.db2", CurrencyContainerStore);
    load_if_exists!("Heirloom.db2", HeirloomStore);
    load_if_exists!("Toy.db2", ToyStore);
    load_if_exists!("TransmogHoliday.db2", TransmogHolidayStore);
    load_if_exists!("TransmogSet.db2", TransmogSetStore);
    load_if_exists!("TransmogSetGroup.db2", TransmogSetGroupStore);
    load_if_exists!("TransmogSetItem.db2", TransmogSetItemStore);
}
