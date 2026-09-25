//! Condition snapshot borrows regression scenarios, part 1 of 3.
//!
//! Moved out of the conditions.rs root under #656; every test is unchanged.

use super::*;

#[test]
fn condition_source_info_uses_first_non_null_target_map_like_cpp() {
    let target1 = world_object(571, 2);
    let target2 = world_object(1, 9);

    let info = ConditionSourceInfo::from_targets(None, Some(&target1), Some(&target2));

    assert_eq!(info.condition_targets[0].map(WorldObject::map_id), None);
    assert_eq!(
        info.condition_targets[1].map(WorldObject::map_id),
        Some(571)
    );
    assert_eq!(info.condition_map, Some(ConditionMapRef::new(571, 2)));
    assert!(info.last_failed_condition.is_none());
}

#[test]
fn condition_source_info_map_constructor_matches_cpp() {
    let info = ConditionSourceInfo::from_map(ConditionMapRef::new(530, 7));

    assert!(info.condition_targets.iter().all(Option::is_none));
    assert_eq!(info.condition_map, Some(ConditionMapRef::new(530, 7)));
    assert!(info.last_failed_condition.is_none());
}

#[test]
fn condition_source_info_tracks_last_failed_condition_like_cpp() {
    let condition = Condition::default();
    let mut info = ConditionSourceInfo::from_targets(None, None, None);

    info.mark_failed_like_cpp(&condition);

    assert!(std::ptr::eq(
        info.last_failed_condition.unwrap(),
        &condition
    ));
}

#[test]
fn global_condition_mgr_store_can_be_installed_and_replaced_like_cpp_reload_foundation() {
    clear_condition_mgr_store_like_cpp();
    assert!(condition_mgr_store_like_cpp().is_none());

    let first = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
        Condition {
            source_type: ConditionSourceType::Phase,
            source_entry: 10,
            condition_type: ConditionType::None,
            ..Condition::default()
        },
    ]));
    set_condition_mgr_store_like_cpp(Arc::clone(&first));
    assert_eq!(
        condition_mgr_store_like_cpp()
            .as_ref()
            .map(|store| store.bucket_count()),
        Some(1)
    );

    let second = Arc::new(ConditionEntriesByTypeStore::default());
    set_condition_mgr_store_like_cpp(Arc::clone(&second));
    assert!(Arc::ptr_eq(
        &condition_mgr_store_like_cpp().expect("condition store installed"),
        &second
    ));

    clear_condition_mgr_store_like_cpp();
}

#[test]
fn basic_condition_meets_map_zone_area_and_negative_like_cpp() {
    let mut target = world_object(571, 2);
    target.set_zone_and_area(67, 123);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    let active_events = [77];
    let world_states = [ConditionWorldStateSnapshot { id: 88, value: -5 }];
    let instance_data = [ConditionMapDataSnapshot { id: 1, value: 42 }];
    let instance_data64 = [ConditionMapDataSnapshot { id: 2, value: 84 }];
    let boss_states = [ConditionMapDataSnapshot { id: 3, value: 2 }];
    info.set_map_state_snapshot(ConditionMapStateSnapshot {
        active_event_ids: &active_events,
        world_states: &world_states,
        difficulty_id: 23,
        instance_data: &instance_data,
        instance_data64: &instance_data64,
        boss_states: &boss_states,
        scenario_step_id: Some(99),
    });

    let map_condition = Condition {
        condition_type: ConditionType::MapId,
        condition_value1: 571,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&map_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let zone_condition = Condition {
        condition_type: ConditionType::ZoneId,
        condition_value1: 67,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&zone_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let active_event_condition = Condition {
        condition_type: ConditionType::ActiveEvent,
        condition_value1: 77,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&active_event_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let world_state_condition = Condition {
        condition_type: ConditionType::WorldState,
        condition_value1: 88,
        condition_value2: (-5_i32) as u32,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&world_state_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let difficulty_condition = Condition {
        condition_type: ConditionType::DifficultyId,
        condition_value1: 23,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&difficulty_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let instance_data_condition = Condition {
        condition_type: ConditionType::InstanceInfo,
        condition_value1: 1,
        condition_value2: 42,
        condition_value3: ConditionInstanceInfo::Data as u32,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&instance_data_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let instance_data64_condition = Condition {
        condition_type: ConditionType::InstanceInfo,
        condition_value1: 2,
        condition_value2: 84,
        condition_value3: ConditionInstanceInfo::Data64 as u32,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&instance_data64_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let boss_state_condition = Condition {
        condition_type: ConditionType::InstanceInfo,
        condition_value1: 3,
        condition_value2: 2,
        condition_value3: ConditionInstanceInfo::BossState as u32,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&boss_state_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let scenario_step_condition = Condition {
        condition_type: ConditionType::ScenarioStep,
        condition_value1: 99,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&scenario_step_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let area_condition = Condition {
        condition_type: ConditionType::AreaId,
        condition_value1: 999,
        negative_condition: true,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&area_condition, &mut info, |current, required| {
            current == 123 && required == 999
        }),
        ConditionMeetResult::Evaluated(false)
    );
    assert!(std::ptr::eq(
        info.last_failed_condition.unwrap(),
        &area_condition
    ));
}

#[test]
fn basic_condition_meets_object_type_entry_mask_and_phasing_like_cpp() {
    let mut target = world_object(571, 2);
    target.object_mut().set_entry(1001);
    target
        .phase_shift_mut()
        .add_phase_like_cpp(55, PhaseFlags::NONE, 1);
    target.phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_spawn_id_target_snapshot(0, 42);
    info.set_private_object_target_snapshot(0, true);
    info.set_string_id_target_snapshot(0, &["template-id", "spawn-id", "script-id"]);

    let entry_condition = Condition {
        condition_type: ConditionType::ObjectEntryGuid,
        condition_value1: TypeId::Unit as u32,
        condition_value2: 1001,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&entry_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let spawn_condition = Condition {
        condition_type: ConditionType::ObjectEntryGuid,
        condition_value1: TypeId::Unit as u32,
        condition_value2: 1001,
        condition_value3: 42,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&spawn_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let private_condition = Condition {
        condition_type: ConditionType::PrivateObject,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&private_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let string_id_condition = Condition {
        condition_type: ConditionType::StringId,
        condition_string_value1: "spawn-id".to_string(),
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&string_id_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let mask_condition = Condition {
        condition_type: ConditionType::TypeMask,
        condition_value1: TypeMask::UNIT.bits(),
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&mask_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let phase_condition = Condition {
        condition_type: ConditionType::PhaseId,
        condition_value1: 55,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&phase_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let terrain_condition = Condition {
        condition_type: ConditionType::TerrainSwap,
        condition_value1: 609,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&terrain_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );
}

#[test]
fn basic_condition_meets_distance_to_uses_cpp_target_and_combat_reach_distance() {
    let mut target0 = world_object(571, 2);
    let mut target1 = world_object(571, 2);
    target0.relocate(Position::xyz(0.0, 0.0, 0.0));
    target1.relocate(Position::xyz(3.0, 4.0, 0.0));
    target0.set_combat_reach(1.0);
    target1.set_combat_reach(1.0);

    let mut info = ConditionSourceInfo::from_targets(Some(&target0), Some(&target1), None);
    let condition = Condition {
        condition_type: ConditionType::DistanceTo,
        condition_value1: 1,
        condition_value2: 3,
        condition_value3: ComparisonType::LowEq as u32,
        ..Condition::default()
    };

    assert_eq!(
        condition_meets_basic_like_cpp(&condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );
}

#[test]
fn basic_condition_meets_relation_self_like_cpp() {
    let target = world_object(571, 2);
    let other = world_object(571, 2);
    let self_condition = Condition {
        condition_type: ConditionType::RelationTo,
        condition_value1: 1,
        condition_value2: RelationType::SelfRelation as u32,
        ..Condition::default()
    };

    let mut same_info = ConditionSourceInfo::from_targets(Some(&target), Some(&target), None);
    assert_eq!(
        condition_meets_basic_like_cpp(&self_condition, &mut same_info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let mut other_info = ConditionSourceInfo::from_targets(Some(&target), Some(&other), None);
    assert_eq!(
        condition_meets_basic_like_cpp(&self_condition, &mut other_info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );

    let party_condition = Condition {
        condition_type: ConditionType::RelationTo,
        condition_value1: 1,
        condition_value2: RelationType::InParty as u32,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&party_condition, &mut other_info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
}

#[test]
fn basic_condition_meets_relation_reaction_and_nearby_snapshots_like_cpp() {
    let target = world_object(571, 2);
    let other = world_object(571, 2);
    let relations = [ConditionUnitRelationSnapshot {
        to_target_index: 1,
        in_party: true,
        in_raid_or_party: true,
        owned_by: true,
        passenger_of: true,
        created_by: true,
        reaction: 4,
    }];
    let nearby_creatures = [
        ConditionNearbyCreatureSnapshot {
            entry: 1001,
            distance: 10.0,
            is_alive: true,
        },
        ConditionNearbyCreatureSnapshot {
            entry: 1002,
            distance: 5.0,
            is_alive: false,
        },
    ];
    let nearby_gameobjects = [ConditionNearbyGameObjectSnapshot {
        entry: 2001,
        distance: 12.0,
    }];
    let mut info = ConditionSourceInfo::from_targets(Some(&target), Some(&other), None);
    info.set_unit_relation_target_snapshot(0, &relations);
    info.set_nearby_creature_target_snapshot(0, &nearby_creatures);
    info.set_nearby_gameobject_target_snapshot(0, &nearby_gameobjects);

    let conditions = vec![
        Condition {
            condition_type: ConditionType::RelationTo,
            condition_value1: 1,
            condition_value2: RelationType::InParty as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::RelationTo,
            condition_value1: 1,
            condition_value2: RelationType::OwnedBy as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::RelationTo,
            condition_value1: 1,
            condition_value2: RelationType::PassengerOf as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::RelationTo,
            condition_value1: 1,
            condition_value2: RelationType::CreatedBy as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ReactionTo,
            condition_value1: 1,
            condition_value2: 1 << 4,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::NearCreature,
            condition_value1: 1001,
            condition_value2: 10,
            condition_value3: 0,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::NearCreature,
            condition_value1: 1002,
            condition_value2: 5,
            condition_value3: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::NearGameObject,
            condition_value1: 2001,
            condition_value2: 12,
            ..Condition::default()
        },
    ];

    for condition in &conditions {
        assert_eq!(
            condition_meets_basic_like_cpp(condition, &mut info, |_, _| false),
            ConditionMeetResult::Evaluated(true),
            "{condition:?}"
        );
    }
}

#[test]
fn basic_condition_meets_unit_snapshot_class_race_level_and_life_like_cpp() {
    let target = world_object(571, 2);
    let aura_effects = [ConditionAuraEffectSnapshot {
        spell_id: 17,
        effect_index: 1,
    }];
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_unit_target_snapshot(
        0,
        ConditionUnitSnapshot {
            level: 70,
            health: 750,
            max_health: 1000,
            class_mask: 1 << (2 - 1),
            race: 4,
            creature_type: Some(7),
            is_alive: true,
            is_charmed: true,
            in_water: true,
            unit_state: 0x20,
            stand_state: 8,
        },
    );
    info.set_unit_aura_target_snapshot(0, &aura_effects);

    let conditions = vec![
        Condition {
            condition_type: ConditionType::Aura,
            condition_value1: 17,
            condition_value2: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Class,
            condition_value1: 1 << (2 - 1),
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Race,
            condition_value1: 1 << (4 - 1),
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Level,
            condition_value1: 60,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Alive,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::HpVal,
            condition_value1: 700,
            condition_value2: ComparisonType::High as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::HpPct,
            condition_value1: 75,
            condition_value2: ComparisonType::Eq as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::UnitState,
            condition_value1: 0x20,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::InWater,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::CreatureType,
            condition_value1: 7,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Charmed,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::StandState,
            condition_value1: 0,
            condition_value2: UnitStandStateType::Kneel as u32,
            ..Condition::default()
        },
    ];

    for condition in &conditions {
        assert_eq!(
            condition_meets_basic_like_cpp(condition, &mut info, |_, _| false),
            ConditionMeetResult::Evaluated(true),
            "{condition:?}"
        );
    }
}

#[test]
fn basic_condition_meets_player_snapshot_branches_like_cpp() {
    let target = player_object(571, 2);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_player_target_snapshot(
        0,
        ConditionPlayerSnapshot {
            team: 469,
            native_gender: 1,
            drunken_state: 2,
            can_be_game_master: true,
            is_game_master: false,
            pet_type: Some(2),
            is_in_flight: true,
        },
    );

    let conditions = vec![
        Condition {
            condition_type: ConditionType::Team,
            condition_value1: 469,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Gender,
            condition_value1: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::DrunkenState,
            condition_value1: 2,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::GameMaster,
            condition_value1: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::PetType,
            condition_value1: 1 << 2,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Taxi,
            ..Condition::default()
        },
    ];

    for condition in &conditions {
        assert_eq!(
            condition_meets_basic_like_cpp(condition, &mut info, |_, _| false),
            ConditionMeetResult::Evaluated(true),
            "{condition:?}"
        );
    }
}

#[test]
fn basic_condition_meets_player_progression_snapshot_branches_like_cpp() {
    let target = player_object(571, 2);
    let items = [ConditionItemCountSnapshot {
        item_id: 100,
        count: 2,
        bank_count: 3,
    }];
    let equipped_item_or_gem_ids = [200];
    let skills = [ConditionSkillSnapshot {
        skill_id: 300,
        base_value: 75,
    }];
    let spell_ids = [400];
    let achievement_ids = [500];
    let reputations = [ConditionReputationSnapshot {
        faction_id: 550,
        rank: 5,
    }];
    let title_ids = [600];
    let battle_pet_counts = [ConditionBattlePetCountSnapshot {
        species_id: 700,
        count: 4,
    }];
    let active_scene_ids = [900];
    let realm_achievement_ids = [800];
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_player_progression_target_snapshot(
        0,
        ConditionPlayerProgressionSnapshot {
            items: &items,
            equipped_item_or_gem_ids: &equipped_item_or_gem_ids,
            skills: &skills,
            spell_ids: &spell_ids,
            achievement_ids: &achievement_ids,
            reputations: &reputations,
            title_ids: &title_ids,
            battle_pet_counts: &battle_pet_counts,
            active_scene_ids: &active_scene_ids,
        },
    );
    info.set_realm_achievement_ids(&realm_achievement_ids);

    let conditions = vec![
        Condition {
            condition_type: ConditionType::Item,
            condition_value1: 100,
            condition_value2: 5,
            condition_value3: 1,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ItemEquipped,
            condition_value1: 200,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Skill,
            condition_value1: 300,
            condition_value2: 75,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Spell,
            condition_value1: 400,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Achievement,
            condition_value1: 500,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::ReputationRank,
            condition_value1: 550,
            condition_value2: 1 << 5,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::Title,
            condition_value1: 600,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::BattlePetCount,
            condition_value1: 700,
            condition_value2: 4,
            condition_value3: ComparisonType::HighEq as u32,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::RealmAchievement,
            condition_value1: 800,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::SceneInProgress,
            condition_value1: 900,
            ..Condition::default()
        },
    ];

    for condition in &conditions {
        assert_eq!(
            condition_meets_basic_like_cpp(condition, &mut info, |_, _| false),
            ConditionMeetResult::Evaluated(true),
            "{condition:?}"
        );
    }

    let without_bank = Condition {
        condition_type: ConditionType::Item,
        condition_value1: 100,
        condition_value2: 5,
        condition_value3: 0,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&without_bank, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
}

#[test]
fn basic_condition_meets_player_quest_snapshot_branches_like_cpp() {
    let target = player_object(571, 2);
    let statuses = [
        ConditionQuestStatusSnapshot {
            quest_id: 10,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        },
        ConditionQuestStatusSnapshot {
            quest_id: 20,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
        },
        ConditionQuestStatusSnapshot {
            quest_id: 30,
            status: QUEST_STATUS_FAILED_LIKE_CPP,
        },
    ];
    let objective_progress = [ConditionQuestObjectiveProgressSnapshot {
        quest_id: 10,
        objective_id: 900,
        counter: 4,
    }];
    let rewarded_quest_ids = [40];
    let daily_quest_ids = [50];
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_player_quest_target_snapshot(
        0,
        ConditionPlayerQuestSnapshot {
            statuses: &statuses,
            objective_progress: &objective_progress,
            rewarded_quest_ids: &rewarded_quest_ids,
            daily_quest_ids: &daily_quest_ids,
        },
    );

    let conditions = vec![
        Condition {
            condition_type: ConditionType::QuestTaken,
            condition_value1: 10,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestComplete,
            condition_value1: 20,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestNone,
            condition_value1: 999,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestRewarded,
            condition_value1: 40,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestState,
            condition_value1: 30,
            condition_value2: 1 << QUEST_STATUS_FAILED_LIKE_CPP,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestState,
            condition_value1: 40,
            condition_value2: 1 << QUEST_STATUS_REWARDED_LIKE_CPP,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: 900,
            condition_value3: 4,
            ..Condition::default()
        },
        Condition {
            condition_type: ConditionType::DailyQuestDone,
            condition_value1: 50,
            ..Condition::default()
        },
    ];

    for condition in &conditions {
        assert_eq!(
            condition_meets_basic_like_cpp(condition, &mut info, |_, _| false),
            ConditionMeetResult::Evaluated(true),
            "{condition:?}"
        );
    }

    let rewarded_is_not_complete = Condition {
        condition_type: ConditionType::QuestComplete,
        condition_value1: 40,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&rewarded_is_not_complete, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
}

#[test]
fn basic_condition_meets_stand_state_modes_like_cpp() {
    let target = world_object(571, 2);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_unit_target_snapshot(
        0,
        ConditionUnitSnapshot {
            level: 1,
            health: 1,
            max_health: 1,
            class_mask: 1,
            race: 1,
            creature_type: None,
            is_alive: true,
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::SitChair as u32,
        },
    );

    let sit_mode = Condition {
        condition_type: ConditionType::StandState,
        condition_value1: 1,
        condition_value2: 1,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&sit_mode, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let stand_mode = Condition {
        condition_type: ConditionType::StandState,
        condition_value1: 1,
        condition_value2: 0,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&stand_mode, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
}
