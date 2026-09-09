//! Condition entry model and load reporting regression scenarios, part 2 of 4.
//!
//! Moved out of the conditions.rs root under #638; every test is unchanged.

use super::*;

#[test]
fn condition_external_type_validation_uses_loaded_stores_like_cpp() {
    let item_store = crate::ItemStore::from_records([crate::ItemRecord {
        id: 100,
        class_id: 0,
        subclass_id: 0,
        material: 0,
        inventory_type: 0,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }]);
    let mut spell_store = crate::SpellStore::new();
    spell_store.insert(200, spell_info(200));
    let area_store = crate::AreaTableStore::from_entries([
        crate::AreaTableEntry {
            id: 300,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        crate::AreaTableEntry {
            id: 301,
            continent_id: 0,
            parent_area_id: 300,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: crate::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP,
        },
    ]);
    let skill_line_store = crate::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
        [crate::SkillLineEntry {
            id: 401,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: 0,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }],
        [400],
    );
    let map_store = crate::MapStore::from_entries([crate::MapEntry {
        id: 500,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        instance_type: 0,
        expansion_id: 0,
        flags1: 0,
        flags2: 0,
    }]);
    let phase_store = crate::PhaseStore::from_entries([crate::PhaseEntry { id: 600, flags: 0 }]);
    let quest_store = crate::quest::QuestStore::from_quests_like_cpp([quest_template(
        700,
        vec![quest_objective(800, 0, 5), quest_objective(801, 10, 99)],
    )]);
    let difficulty_store = crate::DifficultyStore::from_ids([900]);
    let faction_store = crate::Db2IdStore::from_ids("Faction.db2", [910]);
    let achievement_store = crate::Db2IdStore::from_ids("Achievement.db2", [920]);
    let char_titles_store = crate::Db2IdStore::from_ids("CharTitles.db2", [930]);
    let battle_pet_species_store = crate::Db2IdStore::from_ids("BattlePetSpecies.db2", [940]);
    let scenario_step_store = crate::Db2IdStore::from_ids("ScenarioStep.db2", [950]);
    let scene_script_package_store = crate::Db2IdStore::from_ids("SceneScriptPackage.db2", [960]);
    let player_condition_store =
        crate::PlayerConditionStore::from_entries([crate::PlayerConditionEntry {
            id: 970,
            ..crate::PlayerConditionEntry::default()
        }]);
    let creature_template_store = crate::WorldIdStore::from_ids("creature_template", [980, 981]);
    let gameobject_template_store =
        crate::WorldIdStore::from_ids("gameobject_template", [990, 991]);
    let creature_spawn_store = crate::WorldSpawnIdStore::from_entries("creature", [(1000, 980)]);
    let gameobject_spawn_store =
        crate::WorldSpawnIdStore::from_entries("gameobject", [(1001, 990)]);
    let active_event_store = crate::WorldIdStore::from_ids("game_event", [1100]);
    let world_state_store = crate::WorldIdStore::from_ids("world_state", [1200]);
    let stores = ConditionExternalValidationStoresLikeCpp {
        item_store: Some(&item_store),
        spell_store: Some(&spell_store),
        area_table_store: Some(&area_store),
        skill_line_store: Some(&skill_line_store),
        map_store: Some(&map_store),
        phase_store: Some(&phase_store),
        quest_store: Some(&quest_store),
        difficulty_store: Some(&difficulty_store),
        faction_store: Some(&faction_store),
        achievement_store: Some(&achievement_store),
        char_titles_store: Some(&char_titles_store),
        battle_pet_species_store: Some(&battle_pet_species_store),
        scenario_step_store: Some(&scenario_step_store),
        scene_script_package_store: Some(&scene_script_package_store),
        player_condition_store: Some(&player_condition_store),
        creature_template_store: Some(&creature_template_store),
        gameobject_template_store: Some(&gameobject_template_store),
        creature_spawn_store: Some(&creature_spawn_store),
        gameobject_spawn_store: Some(&gameobject_spawn_store),
        active_event_store: Some(&active_event_store),
        world_state_store: Some(&world_state_store),
        max_skill_value: Some(450),
        ..ConditionExternalValidationStoresLikeCpp::default()
    };

    for condition in [
        Condition {
            condition_type: ConditionType::Item,
            condition_value1: 100,
            condition_value2: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Aura,
            condition_value1: 200,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ZoneId,
            condition_value1: 300,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Skill,
            condition_value1: 400,
            condition_value2: 450,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::MapId,
            condition_value1: 500,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::PhaseId,
            condition_value1: 600,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestRewarded,
            condition_value1: 700,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: 800,
            condition_value3: 5,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: 801,
            condition_value3: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::DifficultyId,
            condition_value1: 900,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ReputationRank,
            condition_value1: 910,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Achievement,
            condition_value1: 920,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::RealmAchievement,
            condition_value1: 920,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Title,
            condition_value1: 930,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::BattlePetCount,
            condition_value1: 940,
            condition_value3: 0,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ScenarioStep,
            condition_value1: 950,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::SceneInProgress,
            condition_value1: 960,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::PlayerCondition,
            condition_value1: 970,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::NearCreature,
            condition_value1: 980,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::Unit as u32,
            condition_value2: 980,
            condition_value3: 1000,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::NearGameObject,
            condition_value1: 990,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::GameObject as u32,
            condition_value2: 990,
            condition_value3: 1001,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ActiveEvent,
            condition_value1: 1100,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::WorldState,
            condition_value1: 1200,
            ..Condition::default()
        },
    ] {
        assert_eq!(
            validate_condition_type_external_like_cpp(&condition, stores),
            Ok(())
        );
    }

    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ItemEquipped,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingItem { item_id: 999, .. })
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ZoneId,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingArea {
            condition_type: ConditionType::ZoneId,
            area_id: 999
        })
    ));
    assert_eq!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::AreaId,
                condition_value1: 301,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ZoneId,
                condition_value1: 301,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::ZoneIdUsesSubzone(301))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::Skill,
                condition_value1: 400,
                condition_value2: 451,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::SkillValueAboveConfigMax {
                value: 451,
                max: 450,
                ..
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::Skill,
                condition_value1: 401,
                condition_value2: 1,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingSkill(401))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::QuestComplete,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingQuest { quest_id: 999, .. })
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::QuestObjectiveProgress,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingQuestObjective(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::QuestObjectiveProgress,
                condition_value1: 801,
                condition_value3: 2,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::QuestObjectiveCountAboveLimit {
                objective_id: 801,
                count: 2,
                limit: 1
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::DifficultyId,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingDifficulty(
            999
        ))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ReputationRank,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingFaction(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::RealmAchievement,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::NonExistingAchievement {
                condition_type: ConditionType::RealmAchievement,
                achievement_id: 999
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::Title,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingTitle(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::BattlePetCount,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingBattlePetSpecies(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ScenarioStep,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingScenarioStep(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::SceneInProgress,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingSceneScriptPackage(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::PlayerCondition,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingPlayerCondition(999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::NearCreature,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::NonExistingCreatureTemplate {
                condition_type: ConditionType::NearCreature,
                entry: 999
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::GameObject as u32,
                condition_value2: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::NonExistingGameObjectTemplate {
                condition_type: ConditionType::ObjectEntryGuid,
                entry: 999
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::Unit as u32,
                condition_value3: 9999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingCreatureGuid(9999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::Unit as u32,
                condition_value2: 981,
                condition_value3: 1000,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::CreatureGuidEntryMismatch {
                guid: 1000,
                expected_entry: 981,
                actual_entry: 980
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::GameObject as u32,
                condition_value3: 9999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingGameObjectGuid(9999))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::GameObject as u32,
                condition_value2: 991,
                condition_value3: 1001,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionTypeValidationErrorLikeCpp::GameObjectGuidEntryMismatch {
                guid: 1001,
                expected_entry: 991,
                actual_entry: 990
            }
        )
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::ActiveEvent,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingActiveEvent(
            999
        ))
    ));
    assert!(matches!(
        validate_condition_type_external_like_cpp(
            &Condition {
                condition_type: ConditionType::WorldState,
                condition_value1: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionTypeValidationErrorLikeCpp::NonExistingWorldState(
            999
        ))
    ));
}
