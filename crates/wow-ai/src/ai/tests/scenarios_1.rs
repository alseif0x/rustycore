//! Creature AI state machines regression scenarios, part 1 of 2.
//!
//! Moved out of the lib.rs root under #658; every test is unchanged.

use super::*;

#[test]
fn creature_ai_runtime_rng_replaces_timer_seeded_damage_like_cpp() {
    let mut creature = CreatureAI::new(
        ObjectGuid::EMPTY,
        1,
        Position::ZERO,
        100,
        1,
        3,
        7,
        0.0,
        1,
        35,
        0,
        0,
        0,
        0,
        0,
        None,
        0,
    );
    creature.seed_runtime_rng_like_cpp(0xA141_BEEF);

    let rolls: Vec<u32> = (0..16).map(|_| creature.roll_damage()).collect();

    assert!(rolls.iter().all(|roll| (3..=7).contains(roll)));
    assert!(
        rolls.iter().any(|roll| *roll != rolls[0]),
        "damage rolls should come from owned RNG, not a constant timer seed: {rolls:?}"
    );
}

#[test]
fn creature_ai_wander_rng_matches_cpp_random_movement_bounds() {
    let mut creature = CreatureAI::new(
        ObjectGuid::EMPTY,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        100,
        1,
        3,
        7,
        0.0,
        1,
        35,
        0,
        0,
        0,
        0,
        0,
        None,
        0,
    );
    creature.wander_radius = 12.0;
    creature.seed_runtime_rng_like_cpp(0x5757);

    for _ in 0..24 {
        let dst = creature.pick_wander_destination();
        let dist = creature.home_pos.distance(&dst);
        assert!(
            dist <= creature.wander_radius + f32::EPSILON,
            "wander destination {dst:?} was {dist} yd from home"
        );
    }

    for _ in 0..24 {
        creature.reset_wander_timer();
        assert!(
            (4_000..=10_000).contains(&creature.wander_delay_ms),
            "C++ RandomMovementGenerator pauses with urand(4, 10) seconds"
        );
    }
}

#[test]
fn creature_ai_selector_pet_overrides_script_and_ai_name_like_cpp() {
    let input = CreatureAiSelectionInputLikeCpp {
        is_pet: true,
        script_name: "boss_should_not_win".to_string(),
        script_can_create_creature_ai: true,
        ai_name: "SmartAI".to_string(),
        ..selector_input()
    };

    assert_eq!(
        select_creature_ai_like_cpp(&input),
        CreatureAiKindLikeCpp::PetAI
    );
}

#[test]
fn creature_ai_selector_uses_script_before_ai_name_like_cpp() {
    let input = CreatureAiSelectionInputLikeCpp {
        script_name: "npc_scripted".to_string(),
        script_can_create_creature_ai: true,
        ai_name: "AggressorAI".to_string(),
        ..selector_input()
    };

    assert_eq!(
        select_creature_ai_like_cpp(&input),
        CreatureAiKindLikeCpp::ScriptedAI("npc_scripted".to_string())
    );
}

#[test]
fn creature_ai_selector_uses_registered_ai_name_before_permits_like_cpp() {
    let input = CreatureAiSelectionInputLikeCpp {
        ai_name: "TurretAI".to_string(),
        is_vehicle: true,
        ..selector_input()
    };

    assert_eq!(
        select_creature_ai_like_cpp(&input),
        CreatureAiKindLikeCpp::TurretAI
    );
}

#[test]
fn creature_ai_selector_falls_back_to_stock_permits_like_cpp() {
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_vehicle: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::VehicleAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_trigger: true,
            first_spell_id: 133,
            is_vehicle: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::TriggerAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_trigger: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::NullCreatureAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            has_spellclick_npc_flag: true,
            is_guard: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::NullCreatureAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_guard: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::GuardAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_controllable_guardian: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::AggressorAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_controllable_guardian: true,
            is_civilian: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::PetAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_civilian: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::ReactorAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
            is_neutral_to_all: true,
            ..selector_input()
        }),
        CreatureAiKindLikeCpp::ReactorAI
    );
    assert_eq!(
        select_creature_ai_like_cpp(&selector_input()),
        CreatureAiKindLikeCpp::AggressorAI
    );
}

#[test]
fn creature_ai_selector_preserves_unknown_ai_name_as_unrepresented_like_cpp() {
    let input = CreatureAiSelectionInputLikeCpp {
        ai_name: "CustomPrivateAI".to_string(),
        ..selector_input()
    };

    assert_eq!(
        select_creature_ai_like_cpp(&input),
        CreatureAiKindLikeCpp::UnknownNamedAI("CustomPrivateAI".to_string())
    );
}

#[test]
fn creature_ai_can_attack_defaults_true_for_stock_ai_like_cpp() {
    for ai_kind in [
        CreatureAiKindLikeCpp::AggressorAI,
        CreatureAiKindLikeCpp::ReactorAI,
        CreatureAiKindLikeCpp::GuardAI,
        CreatureAiKindLikeCpp::SmartAI,
        CreatureAiKindLikeCpp::VehicleAI,
        CreatureAiKindLikeCpp::UnknownNamedAI("CustomPrivateAI".to_string()),
    ] {
        assert!(
            creature_ai_can_attack_like_cpp(
                &ai_kind,
                &CreatureAiCanAttackInputLikeCpp {
                    target_within_turret_combat_range: false,
                    target_within_turret_min_range: true,
                    boss_boundary_contains_target: None,
                },
            ),
            "{ai_kind:?} should inherit UnitAI::CanAIAttack == true"
        );
    }
}

#[test]
fn creature_ai_can_attack_applies_turret_range_override_like_cpp() {
    assert!(creature_ai_can_attack_like_cpp(
        &CreatureAiKindLikeCpp::TurretAI,
        &CreatureAiCanAttackInputLikeCpp {
            target_within_turret_combat_range: true,
            target_within_turret_min_range: false,
            boss_boundary_contains_target: None,
        },
    ));
    assert!(!creature_ai_can_attack_like_cpp(
        &CreatureAiKindLikeCpp::TurretAI,
        &CreatureAiCanAttackInputLikeCpp {
            target_within_turret_combat_range: false,
            target_within_turret_min_range: false,
            boss_boundary_contains_target: None,
        },
    ));
    assert!(!creature_ai_can_attack_like_cpp(
        &CreatureAiKindLikeCpp::TurretAI,
        &CreatureAiCanAttackInputLikeCpp {
            target_within_turret_combat_range: true,
            target_within_turret_min_range: true,
            boss_boundary_contains_target: None,
        },
    ));
}

#[test]
fn creature_ai_can_attack_applies_boss_boundary_override_like_cpp() {
    assert!(!creature_ai_can_attack_like_cpp(
        &CreatureAiKindLikeCpp::ScriptedAI("boss_script".to_string()),
        &CreatureAiCanAttackInputLikeCpp {
            boss_boundary_contains_target: Some(false),
            ..CreatureAiCanAttackInputLikeCpp::default()
        },
    ));
    assert!(creature_ai_can_attack_like_cpp(
        &CreatureAiKindLikeCpp::ScriptedAI("boss_script".to_string()),
        &CreatureAiCanAttackInputLikeCpp {
            boss_boundary_contains_target: Some(true),
            target_within_turret_combat_range: false,
            target_within_turret_min_range: true,
        },
    ));
}

#[test]
fn creature_attack_distance_zero_rate_returns_zero_like_cpp() {
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            aggro_rate: 0.0,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        0.0,
    );
}

#[test]
fn creature_attack_distance_equal_level_subtracts_combat_reach_like_cpp() {
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            creature_combat_reach: 1.5,
            player_level_for_target: 60,
            creature_level_for_target: 60,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        18.5,
    );
}

#[test]
fn creature_attack_distance_applies_level_difference_and_clamps_like_cpp() {
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            player_level_for_target: 20,
            creature_level_for_target: 25,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        25.0,
    );
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            player_level_for_target: 80,
            creature_level_for_target: 1,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        5.0,
    );
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            player_level_for_target: 1,
            creature_level_for_target: 80,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        45.0,
    );
}

#[test]
fn creature_attack_distance_detect_range_auras_are_level_gated_like_cpp() {
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            player_level_for_target: 40,
            creature_level_for_target: 40,
            max_player_level_config: 80,
            creature_detect_range_aura_mod: 3.0,
            player_detected_range_aura_mod: 2.0,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        25.0,
    );
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            player_level_for_target: 80,
            creature_level_for_target: 80,
            max_player_level_config: 80,
            creature_detect_range_aura_mod: 3.0,
            player_detected_range_aura_mod: 2.0,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        20.0,
    );
}

#[test]
fn creature_attack_distance_caps_creatures_above_expansion_max_like_cpp() {
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            player_level_for_target: 70,
            creature_level_for_target: 83,
            expansion_max_level: 80,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        30.0,
    );
}

#[test]
fn creature_attack_distance_preserves_cpp_rate_order_like_cpp() {
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            aggro_rate: 2.0,
            player_level_for_target: 80,
            creature_level_for_target: 80,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        40.0,
    );
    assert_close_like_cpp(
        creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
            aggro_rate: 2.0,
            player_level_for_target: 80,
            creature_level_for_target: 1,
            ..CreatureAttackDistanceInputLikeCpp::default()
        }),
        20.0,
    );
}

#[test]
fn max_level_for_expansion_matches_cpp_shared_defines_like_cpp() {
    assert_eq!(max_level_for_expansion_like_cpp(0), 60);
    assert_eq!(max_level_for_expansion_like_cpp(1), 70);
    assert_eq!(max_level_for_expansion_like_cpp(2), 80);
    assert_eq!(max_level_for_expansion_like_cpp(3), 80);
    assert_eq!(max_level_for_expansion_like_cpp(9), 80);
    assert_eq!(max_level_for_expansion_like_cpp(10), 0);
}

#[test]
fn creature_ai_move_in_line_of_sight_empty_overrides_are_suppressed_like_cpp() {
    for ai_kind in [
        CreatureAiKindLikeCpp::NullCreatureAI,
        CreatureAiKindLikeCpp::TriggerAI,
        CreatureAiKindLikeCpp::ReactorAI,
        CreatureAiKindLikeCpp::PassiveAI,
        CreatureAiKindLikeCpp::PossessedAI,
        CreatureAiKindLikeCpp::CritterAI,
        CreatureAiKindLikeCpp::PetAI,
        CreatureAiKindLikeCpp::TotemAI,
        CreatureAiKindLikeCpp::VehicleAI,
        CreatureAiKindLikeCpp::ScheduledChangeAI,
    ] {
        assert!(
            !creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind),
            "{ai_kind:?} overrides MoveInLineOfSight with no base auto-aggro"
        );
    }
}

#[test]
fn creature_ai_move_in_line_of_sight_base_users_keep_auto_aggro_path_like_cpp() {
    for ai_kind in [
        CreatureAiKindLikeCpp::AggressorAI,
        CreatureAiKindLikeCpp::GuardAI,
        CreatureAiKindLikeCpp::CombatAI,
        CreatureAiKindLikeCpp::TurretAI,
        CreatureAiKindLikeCpp::SmartAI,
        CreatureAiKindLikeCpp::ScriptedAI("npc_scripted".to_string()),
        CreatureAiKindLikeCpp::UnknownNamedAI("CustomAI".to_string()),
    ] {
        assert!(
            creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind),
            "{ai_kind:?} should reach the base MoveInLineOfSight aggro path"
        );
    }
}

#[test]
fn evade_reason_indices_and_text_match_cpp_enumutils() {
    let expected = [
        (
            EvadeReasonLikeCpp::NoHostiles,
            "NoHostiles",
            "the creature's threat list is empty",
        ),
        (
            EvadeReasonLikeCpp::Boundary,
            "Boundary",
            "the creature has moved outside its evade boundary",
        ),
        (
            EvadeReasonLikeCpp::NoPath,
            "NoPath",
            "the creature was unable to reach its target for over 5 seconds",
        ),
        (
            EvadeReasonLikeCpp::SequenceBreak,
            "SequenceBreak",
            "this is a boss and the pre-requisite encounters for engaging it are not defeated yet",
        ),
        (EvadeReasonLikeCpp::Other, "Other", "anything else"),
    ];

    assert_eq!(EvadeReasonLikeCpp::COUNT_LIKE_CPP, expected.len());
    for (index, (reason, constant, description)) in expected.into_iter().enumerate() {
        assert_eq!(reason.to_index_like_cpp(), index);
        assert_eq!(EvadeReasonLikeCpp::from_index_like_cpp(index), Some(reason));
        assert_eq!(reason.constant_like_cpp(), constant);
        assert_eq!(reason.description_like_cpp(), description);
    }
    assert_eq!(
        EvadeReasonLikeCpp::from_index_like_cpp(expected.len()),
        None
    );
}

#[test]
fn enter_evade_plan_returns_none_if_already_evading_like_cpp() {
    let plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
        is_in_evade_mode: true,
        ..CreatureEnterEvadeInputLikeCpp::default()
    });

    assert!(plan.is_none());
}

#[test]
fn enter_evade_plan_dead_creature_only_ends_engagement_like_cpp() {
    let plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
        reason: EvadeReasonLikeCpp::NoPath,
        is_alive: false,
        ..CreatureEnterEvadeInputLikeCpp::default()
    })
    .unwrap();

    assert_eq!(plan.reason, EvadeReasonLikeCpp::NoPath);
    assert!(plan.engagement_over);
    assert!(!plan.remove_auras_on_evade);
    assert!(!plan.combat_stop_with_pets);
    assert!(!plan.reset_ai);
    assert_eq!(plan.movement, CreatureEvadeMovementLikeCpp::NoneVehicle);
}

#[test]
fn enter_evade_plan_targets_home_and_resets_alive_ownerless_creature_like_cpp() {
    let plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
        reason: EvadeReasonLikeCpp::Boundary,
        ..CreatureEnterEvadeInputLikeCpp::default()
    })
    .unwrap();

    assert_eq!(plan.reason, EvadeReasonLikeCpp::Boundary);
    assert!(plan.remove_auras_on_evade);
    assert!(plan.combat_stop_with_pets);
    assert!(plan.clear_tap);
    assert!(plan.reset_player_damage_req);
    assert!(plan.clear_last_damaged_time);
    assert!(plan.clear_cannot_reach_target);
    assert!(plan.clear_spell_focus_target);
    assert!(plan.clear_target);
    assert!(plan.reset_spell_cooldowns);
    assert!(plan.engagement_over);
    assert!(plan.reset_ai);
    assert_eq!(
        plan.movement,
        CreatureEvadeMovementLikeCpp::TargetedHomeAddEvadeState
    );
}

#[test]
fn enter_evade_plan_follows_owner_unless_vehicle_like_cpp() {
    let owner = guid(900);
    let owner_plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
        owner_guid: Some(owner),
        tap_list_not_cleared_on_evade: true,
        ..CreatureEnterEvadeInputLikeCpp::default()
    })
    .unwrap();

    assert_eq!(
        owner_plan.movement,
        CreatureEvadeMovementLikeCpp::FollowOwner {
            owner_guid: owner,
            pet_follow_distance: 1.0,
        }
    );
    assert!(!owner_plan.clear_tap);

    let vehicle_plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
        has_vehicle: true,
        owner_guid: Some(owner),
        ..CreatureEnterEvadeInputLikeCpp::default()
    })
    .unwrap();

    assert_eq!(
        vehicle_plan.movement,
        CreatureEvadeMovementLikeCpp::NoneVehicle
    );
}

#[test]
fn trigger_alert_plans_ai_reaction_and_distract_like_cpp() {
    let plan = creature_trigger_alert_plan_like_cpp(CreatureTriggerAlertInputLikeCpp {
        absolute_angle_to_target: 1.25,
        ..CreatureTriggerAlertInputLikeCpp::default()
    })
    .unwrap();

    assert!(plan.send_ai_reaction_alert);
    assert_eq!(plan.move_distract_ms, 5_000);
    assert_eq!(plan.orientation, 1.25);
}

#[test]
fn trigger_alert_requires_existing_player_target_like_cpp() {
    for input in [
        CreatureTriggerAlertInputLikeCpp {
            target_exists: false,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            target_is_player: false,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
    ] {
        assert!(creature_trigger_alert_plan_like_cpp(input).is_none());
    }
}

#[test]
fn trigger_alert_skips_invalid_creature_states_like_cpp() {
    for input in [
        CreatureTriggerAlertInputLikeCpp {
            creature_is_unit: false,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_is_engaged: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_is_confused: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_is_stunned: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_is_fleeing: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_is_distracted: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
    ] {
        assert!(creature_trigger_alert_plan_like_cpp(input).is_none());
    }
}

#[test]
fn trigger_alert_requires_hostile_acceptable_non_passive_non_civilian_like_cpp() {
    for input in [
        CreatureTriggerAlertInputLikeCpp {
            creature_is_civilian: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_has_react_passive: true,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            creature_is_hostile_to_target: false,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
        CreatureTriggerAlertInputLikeCpp {
            target_acceptable: false,
            ..CreatureTriggerAlertInputLikeCpp::default()
        },
    ] {
        assert!(creature_trigger_alert_plan_like_cpp(input).is_none());
    }
}

#[test]
fn do_zone_in_combat_logs_and_returns_on_non_dungeon_like_cpp() {
    let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
        creature_guid: guid(1),
        map_is_dungeon: false,
        players: vec![zone_player(10, &[11], Some(12))],
    });

    assert!(plan.log_non_dungeon_error);
    assert!(plan.engage_targets.is_empty());
}

#[test]
fn do_zone_in_combat_dungeon_without_players_is_noop_like_cpp() {
    let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
        creature_guid: guid(1),
        map_is_dungeon: true,
        players: Vec::new(),
    });

    assert!(!plan.log_non_dungeon_error);
    assert!(plan.engage_targets.is_empty());
}

#[test]
fn do_zone_in_combat_skips_dead_or_combat_blocked_players_like_cpp() {
    let mut dead = zone_player(10, &[11], Some(12));
    dead.is_alive = false;
    let mut blocked = zone_player(20, &[21], Some(22));
    blocked.can_begin_combat = false;

    let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
        creature_guid: guid(1),
        map_is_dungeon: true,
        players: vec![dead, blocked, zone_player(30, &[], None)],
    });

    assert_eq!(plan.engage_targets, vec![guid(30)]);
}

#[test]
fn do_zone_in_combat_engages_player_controlled_units_and_vehicle_in_order_like_cpp() {
    let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
        creature_guid: guid(1),
        map_is_dungeon: true,
        players: vec![
            zone_player(10, &[11, 12], Some(13)),
            zone_player(20, &[21], None),
        ],
    });

    assert_eq!(
        plan.engage_targets,
        vec![guid(10), guid(11), guid(12), guid(13), guid(20), guid(21)]
    );
}

#[test]
fn select_target_list_uses_current_victim_then_sorted_threat_like_cpp() {
    let mut low_threat = target_candidate(70, 2, 20.0);
    let mut current = target_candidate(71, 1, 30.0);
    current.is_current_victim = true;
    let high_threat = target_candidate(72, 0, 10.0);

    let selected = select_target_list_like_cpp(
        &[low_threat, current, high_threat],
        usize::MAX,
        SelectTargetMethodLikeCpp::MaxThreat,
        0,
        DefaultTargetSelectorLikeCpp::default(),
    );

    assert_eq!(
        selected,
        vec![current.guid, high_threat.guid, low_threat.guid]
    );

    low_threat.is_offline = true;
    let selected = select_target_list_like_cpp(
        &[low_threat, current, high_threat],
        usize::MAX,
        SelectTargetMethodLikeCpp::MaxThreat,
        0,
        DefaultTargetSelectorLikeCpp::default(),
    );

    assert_eq!(
        selected,
        vec![current.guid, high_threat.guid],
        "C++ skips offline sorted threat refs but keeps current victim if present"
    );
}

#[test]
fn select_target_list_min_threat_reverses_prepared_max_threat_order_like_cpp() {
    let mut current = target_candidate(80, 0, 10.0);
    current.is_current_victim = true;
    let middle = target_candidate(81, 1, 10.0);
    let low = target_candidate(82, 2, 10.0);

    let selected = select_target_list_like_cpp(
        &[current, middle, low],
        2,
        SelectTargetMethodLikeCpp::MinThreat,
        0,
        DefaultTargetSelectorLikeCpp::default(),
    );

    assert_eq!(selected, vec![low.guid, middle.guid]);
}

#[test]
fn select_target_list_distance_methods_sort_unsorted_live_threat_like_cpp() {
    let near = target_candidate(90, 2, 5.0);
    let far = target_candidate(91, 1, 30.0);
    let mid = target_candidate(92, 0, 15.0);

    assert_eq!(
        select_target_list_like_cpp(
            &[near, far, mid],
            usize::MAX,
            SelectTargetMethodLikeCpp::MaxDistance,
            0,
            DefaultTargetSelectorLikeCpp::default(),
        ),
        vec![far.guid, mid.guid, near.guid]
    );
    assert_eq!(
        select_target_list_like_cpp(
            &[near, far, mid],
            usize::MAX,
            SelectTargetMethodLikeCpp::MinDistance,
            1,
            DefaultTargetSelectorLikeCpp::default(),
        ),
        vec![mid.guid, far.guid],
        "C++ applies offset after distance sorting"
    );
}

#[test]
fn select_target_list_applies_offset_before_default_selector_like_cpp() {
    let skipped = target_candidate(100, 0, 5.0);
    let kept = target_candidate(101, 1, 5.0);

    let selected = select_target_list_like_cpp(
        &[skipped, kept],
        usize::MAX,
        SelectTargetMethodLikeCpp::MaxThreat,
        1,
        DefaultTargetSelectorLikeCpp {
            dist: 10.0,
            player_only: true,
            with_tank: true,
            aura: 0,
        },
    );

    assert_eq!(selected, vec![kept.guid]);
}
