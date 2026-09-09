//! Condition snapshot borrows regression scenarios, part 2 of 3.
//!
//! Moved out of the conditions.rs root under #656; every test is unchanged.

use super::*;

#[test]
fn basic_condition_meets_creature_type_requires_creature_like_cpp() {
    let target = player_object(571, 2);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_unit_target_snapshot(
        0,
        ConditionUnitSnapshot {
            level: 1,
            health: 1,
            max_health: 1,
            class_mask: 1,
            race: 1,
            creature_type: Some(7),
            is_alive: true,
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::Stand as u32,
        },
    );

    let condition = Condition {
        condition_type: ConditionType::CreatureType,
        condition_value1: 7,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
}

#[test]
fn basic_condition_meets_unit_branches_fail_without_unit_snapshot_like_cpp_tounit_null() {
    let target = world_object(571, 2);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    let condition = Condition {
        condition_type: ConditionType::Alive,
        ..Condition::default()
    };

    assert_eq!(
        condition_meets_basic_like_cpp(&condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
    assert!(std::ptr::eq(
        info.last_failed_condition.unwrap(),
        &condition
    ));
}

#[test]
fn basic_condition_meets_missing_object_returns_false_before_negative_like_cpp() {
    let condition = Condition {
        condition_type: ConditionType::ZoneId,
        condition_value1: 67,
        negative_condition: true,
        ..Condition::default()
    };
    let mut info = ConditionSourceInfo::from_targets(None, None, None);

    assert_eq!(
        condition_meets_basic_like_cpp(&condition, &mut info, |_, _| true),
        ConditionMeetResult::Evaluated(false)
    );
    assert!(info.last_failed_condition.is_none());
}

#[test]
fn basic_condition_meets_reports_unsupported_for_unrepresented_cpp_state() {
    let mut target = world_object(571, 2);
    target.object_mut().set_entry(1001);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);

    let spawn_condition = Condition {
        condition_type: ConditionType::ObjectEntryGuid,
        condition_value1: TypeId::Unit as u32,
        condition_value2: 1001,
        condition_value3: 77,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&spawn_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );

    let unrepresented_condition = Condition {
        condition_type: ConditionType::PlayerCondition,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&unrepresented_condition, &mut info, |_, _| false),
        ConditionMeetResult::Unsupported
    );
}

#[test]
fn basic_condition_meets_player_condition_delegates_to_db2_evaluator_like_cpp() {
    let target = player_object(571, 2);
    let store = PlayerConditionStore::from_entries([PlayerConditionEntry {
        id: 970,
        race_mask: 1 << 0,
        class_mask: 1 << 1,
        gender: 1,
        ..PlayerConditionEntry::default()
    }]);
    let mut info = ConditionSourceInfo::from_targets(Some(&target), None, None);
    info.set_player_condition_store(&store);
    info.set_player_condition_context(
        0,
        PlayerConditionContextLikeCpp {
            race: 1,
            class_mask: 1 << 1,
            gender: 1,
            native_gender: 0,
            ..Default::default()
        },
    );

    let condition = Condition {
        condition_type: ConditionType::PlayerCondition,
        condition_value1: 970,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(true)
    );

    let missing_condition = Condition {
        condition_type: ConditionType::PlayerCondition,
        condition_value1: 971,
        ..Condition::default()
    };
    assert_eq!(
        condition_meets_basic_like_cpp(&missing_condition, &mut info, |_, _| false),
        ConditionMeetResult::Evaluated(false)
    );
}

#[test]
fn object_meet_conditions_uses_cpp_else_group_or_of_and() {
    let conditions = vec![
        Condition {
            else_group: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 1,
            ..Condition::default()
        },
        Condition {
            else_group: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 2,
            ..Condition::default()
        },
        Condition {
            else_group: 1,
            condition_type: ConditionType::Aura,
            condition_value1: 3,
            ..Condition::default()
        },
    ];
    let store = ConditionEntriesByTypeStore::default();
    let mut info = ConditionSourceInfo::from_targets(None, None, None);

    let passed =
        is_object_meet_to_conditions_like_cpp(&mut info, &conditions, &store, |condition, _| {
            condition.condition_value1 != 2
        });

    assert!(passed);
}

#[test]
fn object_meet_conditions_short_circuits_failed_group_like_cpp() {
    let conditions = vec![
        Condition {
            else_group: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 1,
            ..Condition::default()
        },
        Condition {
            else_group: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 2,
            ..Condition::default()
        },
    ];
    let store = ConditionEntriesByTypeStore::default();
    let mut info = ConditionSourceInfo::from_targets(None, None, None);
    let mut checked = Vec::new();

    let passed =
        is_object_meet_to_conditions_like_cpp(&mut info, &conditions, &store, |condition, _| {
            checked.push(condition.condition_value1);
            false
        });

    assert!(!passed);
    assert_eq!(checked, vec![1]);
}

#[test]
fn object_meet_conditions_expands_reference_conditions_like_cpp() {
    let reference_condition = Condition {
        source_type: ConditionSourceType::ReferenceCondition,
        source_group: 55,
        condition_type: ConditionType::Aura,
        condition_value1: 7,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([reference_condition]);
    let conditions = vec![Condition {
        condition_type: ConditionType::None,
        reference_id: 55,
        ..Condition::default()
    }];
    let mut info = ConditionSourceInfo::from_targets(None, None, None);

    let passed =
        is_object_meet_to_conditions_like_cpp(&mut info, &conditions, &store, |condition, _| {
            condition.condition_value1 == 7
        });

    assert!(passed);
}

#[test]
fn not_grouped_conditions_missing_bucket_passes_like_cpp() {
    let store = ConditionEntriesByTypeStore::default();
    let mut info = ConditionSourceInfo::from_targets(None, None, None);

    assert!(is_object_meeting_not_grouped_conditions_like_cpp(
        &store,
        ConditionSourceType::Phase,
        42,
        &mut info,
        |_, _| false,
    ));
}

#[test]
fn not_grouped_conditions_uses_zero_source_group_and_id_like_cpp() {
    let condition = Condition {
        source_type: ConditionSourceType::Phase,
        source_group: 0,
        source_entry: 42,
        source_id: 0,
        condition_type: ConditionType::Aura,
        condition_value1: 10,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition]);
    let mut info = ConditionSourceInfo::from_targets(None, None, None);

    assert!(is_object_meeting_not_grouped_conditions_like_cpp(
        &store,
        ConditionSourceType::Phase,
        42,
        &mut info,
        |condition, _| condition.condition_value1 == 10,
    ));
}

#[test]
fn has_not_grouped_conditions_uses_cpp_zero_group_entry_key() {
    let condition = Condition {
        source_type: ConditionSourceType::Phase,
        source_group: 0,
        source_entry: -1,
        source_id: 0,
        condition_type: ConditionType::Aura,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition]);

    assert!(has_conditions_for_not_grouped_entry_like_cpp(
        &store,
        ConditionSourceType::Phase,
        u32::MAX,
    ));
    assert!(!has_conditions_for_not_grouped_entry_like_cpp(
        &store,
        ConditionSourceType::None,
        u32::MAX,
    ));
}

fn spawn_group_condition(
    spawn_group_id: u32,
    condition_type: ConditionType,
    value1: u32,
    value2: u32,
    value3: u32,
) -> Condition {
    Condition {
        source_type: ConditionSourceType::SpawnGroup,
        source_group: 0,
        source_entry: spawn_group_id as i32,
        source_id: 0,
        condition_type,
        condition_value1: value1,
        condition_value2: value2,
        condition_value3: value3,
        ..Condition::default()
    }
}

#[test]
fn spawn_group_meeting_map_conditions_missing_bucket_passes_like_cpp() {
    let store = ConditionEntriesByTypeStore::default();

    assert!(is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        123,
        ConditionMapRef::new(571, 1),
        None,
        &[],
    ));
}

#[test]
fn spawn_group_meeting_map_conditions_map_id_matches_cpp_bucket_key() {
    let spawn_group_id = 123;
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([spawn_group_condition(
        spawn_group_id,
        ConditionType::MapId,
        571,
        0,
        0,
    )]);

    assert!(is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        None,
        &[],
    ));
    assert!(!is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(530, 1),
        None,
        &[],
    ));
}

#[test]
fn spawn_group_meeting_map_conditions_world_state_snapshot_matches_cpp() {
    let spawn_group_id = 124;
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([spawn_group_condition(
        spawn_group_id,
        ConditionType::WorldState,
        77,
        42,
        0,
    )]);
    let matching_world_states = [ConditionWorldStateSnapshot { id: 77, value: 42 }];
    let non_matching_world_states = [ConditionWorldStateSnapshot { id: 77, value: 7 }];
    let matching_state = ConditionMapStateSnapshot {
        active_event_ids: &[],
        world_states: &matching_world_states,
        difficulty_id: 0,
        instance_data: &[],
        instance_data64: &[],
        boss_states: &[],
        scenario_step_id: None,
    };
    let non_matching_state = ConditionMapStateSnapshot {
        active_event_ids: &[],
        world_states: &non_matching_world_states,
        difficulty_id: 0,
        instance_data: &[],
        instance_data64: &[],
        boss_states: &[],
        scenario_step_id: None,
    };

    assert!(is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        Some(matching_state),
        &[],
    ));
    assert!(!is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        Some(non_matching_state),
        &[],
    ));
}

#[test]
fn spawn_group_meeting_map_conditions_realm_achievement_ids_match_cpp() {
    let spawn_group_id = 125;
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([spawn_group_condition(
        spawn_group_id,
        ConditionType::RealmAchievement,
        9001,
        0,
        0,
    )]);

    assert!(is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        None,
        &[9001],
    ));
    assert!(!is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        None,
        &[42],
    ));
}

#[test]
fn spawn_group_meeting_map_conditions_map_only_unsupported_type_fails_without_panic() {
    let spawn_group_id = 126;
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([spawn_group_condition(
        spawn_group_id,
        ConditionType::Aura,
        1234,
        0,
        0,
    )]);

    assert!(!is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        None,
        &[],
    ));
}

#[test]
fn spawn_group_meeting_map_conditions_reference_conditions_are_expanded_like_cpp() {
    let spawn_group_id = 127;
    let passing_reference_id = 700;
    let failing_reference_id = 701;
    let passing_spawn_group_condition = Condition {
        reference_id: passing_reference_id,
        ..spawn_group_condition(spawn_group_id, ConditionType::None, 0, 0, 0)
    };
    let failing_spawn_group_condition = Condition {
        reference_id: failing_reference_id,
        ..spawn_group_condition(spawn_group_id + 1, ConditionType::None, 0, 0, 0)
    };
    let passing_reference_condition = Condition {
        source_type: ConditionSourceType::ReferenceCondition,
        source_group: passing_reference_id,
        condition_type: ConditionType::MapId,
        condition_value1: 571,
        ..Condition::default()
    };
    let failing_reference_condition = Condition {
        source_type: ConditionSourceType::ReferenceCondition,
        source_group: failing_reference_id,
        condition_type: ConditionType::MapId,
        condition_value1: 530,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        passing_spawn_group_condition,
        failing_spawn_group_condition,
        passing_reference_condition,
        failing_reference_condition,
    ]);

    assert!(is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id,
        ConditionMapRef::new(571, 1),
        None,
        &[],
    ));
    assert!(!is_spawn_group_meeting_map_conditions_like_cpp(
        &store,
        spawn_group_id + 1,
        ConditionMapRef::new(571, 1),
        None,
        &[],
    ));
}

#[test]
fn spell_click_conditions_use_cpp_key_and_target_order() {
    let condition = Condition {
        source_type: ConditionSourceType::SpellClickEvent,
        source_group: 123,
        source_entry: -1,
        source_id: 0,
        condition_type: ConditionType::Aura,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition]);
    let clicker = world_object(571, 1);
    let target = world_object(1, 2);

    assert!(has_conditions_for_spell_click_event_like_cpp(
        &store,
        123,
        u32::MAX,
    ));
    assert!(is_object_meeting_spell_click_conditions_like_cpp(
        &store,
        123,
        u32::MAX,
        Some(&clicker),
        Some(&target),
        |_, source_info| {
            std::ptr::eq(source_info.condition_targets[0].unwrap(), &clicker)
                && std::ptr::eq(source_info.condition_targets[1].unwrap(), &target)
        },
    ));
}

#[test]
fn can_see_spell_click_requires_flag_and_loaded_rows_like_cpp() {
    let spell_click_store = NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 123,
            spell_id: 456,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 123,
        |spell| spell == 456,
    );
    let condition_store = ConditionEntriesByTypeStore::default();
    let context = SpellClickRequirementContextLikeCpp {
        clicker_is_player: true,
        clicker_is_friendly_to_summoner: false,
        clicker_is_in_raid_with_summoner: false,
        clicker_is_in_party_with_summoner: false,
    };

    assert!(!can_see_spell_click_on_like_cpp(
        &spell_click_store,
        &condition_store,
        123,
        0,
        None,
        None,
        context,
        |_, _| true,
    ));
    assert!(!can_see_spell_click_on_like_cpp(
        &spell_click_store,
        &condition_store,
        124,
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        None,
        None,
        context,
        |_, _| true,
    ));
    assert!(can_see_spell_click_on_like_cpp(
        &spell_click_store,
        &condition_store,
        123,
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        None,
        None,
        context,
        |_, _| false,
    ));
}

#[test]
fn can_see_spell_click_stops_on_first_failed_requirement_like_cpp() {
    let spell_click_store = NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::NpcSpellClickRowLikeCpp {
                npc_entry: 123,
                spell_id: 456,
                cast_flags: 0,
                user_type: SPELL_CLICK_USER_PARTY_LIKE_CPP,
            },
            wow_data::NpcSpellClickRowLikeCpp {
                npc_entry: 123,
                spell_id: 457,
                cast_flags: 0,
                user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
            },
        ],
        |entry| entry == 123,
        |spell| matches!(spell, 456 | 457),
    );
    let condition_store = ConditionEntriesByTypeStore::default();
    let context = SpellClickRequirementContextLikeCpp {
        clicker_is_player: true,
        clicker_is_friendly_to_summoner: true,
        clicker_is_in_raid_with_summoner: true,
        clicker_is_in_party_with_summoner: false,
    };
    let mut condition_calls = 0;

    assert!(!can_see_spell_click_on_like_cpp(
        &spell_click_store,
        &condition_store,
        123,
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        None,
        None,
        context,
        |_, _| {
            condition_calls += 1;
            true
        },
    ));
    assert_eq!(condition_calls, 0);
}

#[test]
fn can_see_spell_click_uses_spell_conditions_until_one_passes_like_cpp() {
    let spell_click_store = NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::NpcSpellClickRowLikeCpp {
                npc_entry: 123,
                spell_id: 456,
                cast_flags: 0,
                user_type: SPELL_CLICK_USER_FRIEND_LIKE_CPP,
            },
            wow_data::NpcSpellClickRowLikeCpp {
                npc_entry: 123,
                spell_id: 457,
                cast_flags: 0,
                user_type: SPELL_CLICK_USER_RAID_LIKE_CPP,
            },
        ],
        |entry| entry == 123,
        |spell| matches!(spell, 456 | 457),
    );
    let condition_store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 123,
            source_entry: 456,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
        Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 123,
            source_entry: 457,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
    ]);
    let clicker = player_object(571, 1);
    let target = world_object(571, 1);
    let context = SpellClickRequirementContextLikeCpp {
        clicker_is_player: true,
        clicker_is_friendly_to_summoner: true,
        clicker_is_in_raid_with_summoner: true,
        clicker_is_in_party_with_summoner: false,
    };
    let mut seen_spells = Vec::new();

    assert!(can_see_spell_click_on_like_cpp(
        &spell_click_store,
        &condition_store,
        123,
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        Some(&clicker),
        Some(&target),
        context,
        |condition, source_info| {
            seen_spells.push(condition.source_entry as u32);
            std::ptr::eq(source_info.condition_targets[0].unwrap(), &clicker)
                && std::ptr::eq(source_info.condition_targets[1].unwrap(), &target)
                && condition.source_entry == 457
        },
    ));
    assert_eq!(seen_spells, vec![456, 457]);
}

#[test]
fn vehicle_vendor_and_trainer_conditions_match_cpp_keys_and_targets() {
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        Condition {
            source_type: ConditionSourceType::VehicleSpell,
            source_group: 10,
            source_entry: 20,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
        Condition {
            source_type: ConditionSourceType::NpcVendor,
            source_group: 30,
            source_entry: 40,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
        Condition {
            source_type: ConditionSourceType::TrainerSpell,
            source_group: 50,
            source_entry: 60,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
    ]);
    let player = world_object(571, 1);
    let other = world_object(571, 1);

    assert!(is_object_meeting_vehicle_spell_conditions_like_cpp(
        &store,
        10,
        20,
        Some(&player),
        Some(&other),
        |_, source_info| {
            std::ptr::eq(source_info.condition_targets[0].unwrap(), &player)
                && std::ptr::eq(source_info.condition_targets[1].unwrap(), &other)
        },
    ));
    assert!(is_object_meeting_vendor_item_conditions_like_cpp(
        &store,
        30,
        40,
        Some(&player),
        Some(&other),
        |_, source_info| {
            std::ptr::eq(source_info.condition_targets[0].unwrap(), &player)
                && std::ptr::eq(source_info.condition_targets[1].unwrap(), &other)
        },
    ));
    assert!(is_object_meeting_trainer_spell_conditions_like_cpp(
        &store,
        50,
        60,
        Some(&player),
        |_, source_info| std::ptr::eq(source_info.condition_targets[0].unwrap(), &player),
    ));
    assert!(is_object_meeting_trainer_spell_conditions_like_cpp(
        &store,
        30,
        40,
        Some(&player),
        |_, _| false,
    ));
}

#[test]
fn smart_event_and_visibility_conditions_match_cpp_composite_keys() {
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        Condition {
            source_type: ConditionSourceType::SmartEvent,
            source_group: 8,
            source_entry: -7,
            source_id: 9,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
        Condition {
            source_type: ConditionSourceType::ObjectIdVisibility,
            source_group: 11,
            source_entry: -1,
            source_id: 0,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
    ]);
    let unit = world_object(571, 1);
    let base = world_object(571, 1);

    assert!(is_object_meeting_smart_event_conditions_like_cpp(
        &store,
        -7,
        7,
        9,
        Some(&unit),
        Some(&base),
        |_, source_info| {
            std::ptr::eq(source_info.condition_targets[0].unwrap(), &unit)
                && std::ptr::eq(source_info.condition_targets[1].unwrap(), &base)
        },
    ));
    assert!(
        is_object_meeting_visibility_by_object_id_conditions_like_cpp(
            &store,
            11,
            u32::MAX,
            Some(&unit),
            |_, source_info| std::ptr::eq(source_info.condition_targets[0].unwrap(), &unit),
        )
    );
}

#[test]
fn area_trigger_lookup_uses_server_side_as_cpp_source_entry() {
    let client_condition = Condition {
        source_type: ConditionSourceType::AreaTrigger,
        source_group: 77,
        source_entry: 0,
        condition_type: ConditionType::Aura,
        condition_value1: 10,
        ..Condition::default()
    };
    let server_condition = Condition {
        source_type: ConditionSourceType::AreaTrigger,
        source_group: 77,
        source_entry: 1,
        condition_type: ConditionType::Aura,
        condition_value1: 20,
        ..Condition::default()
    };
    let store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([client_condition, server_condition]);

    assert_eq!(
        conditions_for_area_trigger_like_cpp(&store, 77, false).unwrap()[0].condition_value1,
        10
    );
    assert_eq!(
        conditions_for_area_trigger_like_cpp(&store, 77, true).unwrap()[0].condition_value1,
        20
    );
}

#[test]
fn loot_store_item_conditions_use_cpp_store_entry_item_key_and_looter_target() {
    let condition = Condition {
        source_type: ConditionSourceType::CreatureLootTemplate,
        source_group: 500,
        source_entry: 6948,
        condition_type: ConditionType::Aura,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition]);
    let looter = world_object(571, 1);
    let context = LootStoreItemContext {
        store_kind: LootStoreKind::Creature,
        entry: 500,
        item: LootStoreItem {
            item_id: 6948,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: 1,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
    };

    assert!(is_loot_store_item_meeting_conditions_like_cpp(
        &store,
        context,
        Some(&looter),
        |_, source_info| std::ptr::eq(source_info.condition_targets[0].unwrap(), &looter),
    ));

    let missing_item_context = LootStoreItemContext {
        item: LootStoreItem {
            item_id: 6949,
            ..context.item
        },
        ..context
    };
    assert!(is_loot_store_item_meeting_conditions_like_cpp(
        &store,
        missing_item_context,
        Some(&looter),
        |_, _| false,
    ));
}
