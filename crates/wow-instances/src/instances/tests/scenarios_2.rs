//! Instance lifecycle state regression scenarios, part 2 of 2.
//!
//! Moved out of the lib.rs root under #658; every test is unchanged.

use super::*;

#[test]
fn instance_script_load_persistent_values_like_cpp() {
    let mut script = InstanceScriptBase::new(4, 1);
    script.set_header("TEST");
    script.register_persistent_value_like_cpp("Kills", PersistentInstanceScriptValue::I64(0));
    script.register_persistent_value_like_cpp("Ratio", PersistentInstanceScriptValue::F64(0.0));

    script
        .load_save_data_like_cpp(
            "{\"Header\":\"TEST\",\"BossStates\":[0],\"AdditionalData\":{\"Kills\":9,\"Ratio\":1.25}}",
        )
        .unwrap();

    assert_eq!(
        script.persistent_value("Kills"),
        Some(&PersistentInstanceScriptValue::I64(9))
    );
    assert_eq!(
        script.persistent_value("Ratio"),
        Some(&PersistentInstanceScriptValue::F64(1.25))
    );
}

#[test]
fn instance_script_load_rejects_cpp_error_cases() {
    let mut script = InstanceScriptBase::new(4, 1);
    script.set_header("TEST");
    script.register_persistent_value_like_cpp("Kills", PersistentInstanceScriptValue::I64(0));

    assert_eq!(
        script.load_save_data_like_cpp("{").unwrap_err(),
        InstanceScriptDataLoadError::MalformedJson
    );
    assert_eq!(
        script.load_save_data_like_cpp("[]").unwrap_err(),
        InstanceScriptDataLoadError::RootIsNotAnObject
    );
    assert_eq!(
        script
            .load_save_data_like_cpp("{\"BossStates\":[0]}")
            .unwrap_err(),
        InstanceScriptDataLoadError::MissingHeader
    );
    assert_eq!(
        script
            .load_save_data_like_cpp("{\"Header\":\"BAD\",\"BossStates\":[0]}")
            .unwrap_err(),
        InstanceScriptDataLoadError::UnexpectedHeader
    );
    assert_eq!(
        script
            .load_save_data_like_cpp("{\"Header\":\"TEST\"}")
            .unwrap_err(),
        InstanceScriptDataLoadError::MissingBossStates
    );
    assert_eq!(
        script
            .load_save_data_like_cpp("{\"Header\":\"TEST\",\"BossStates\":{}}")
            .unwrap_err(),
        InstanceScriptDataLoadError::BossStatesIsNotAnArray
    );
    assert_eq!(
        script
            .load_save_data_like_cpp("{\"Header\":\"TEST\",\"BossStates\":[0,0]}")
            .unwrap_err(),
        InstanceScriptDataLoadError::UnknownBoss
    );
    assert_eq!(
        script
            .load_save_data_like_cpp("{\"Header\":\"TEST\",\"BossStates\":[\"x\"]}")
            .unwrap_err(),
        InstanceScriptDataLoadError::BossStateIsNotANumber
    );
    assert_eq!(
        script
            .load_save_data_like_cpp(
                "{\"Header\":\"TEST\",\"BossStates\":[0],\"AdditionalData\":[]}"
            )
            .unwrap_err(),
        InstanceScriptDataLoadError::AdditionalDataIsNotAnObject
    );
    assert_eq!(
        script
            .load_save_data_like_cpp(
                "{\"Header\":\"TEST\",\"BossStates\":[0],\"AdditionalData\":{\"Kills\":\"x\"}}"
            )
            .unwrap_err(),
        InstanceScriptDataLoadError::AdditionalDataUnexpectedValueType
    );
}

#[test]
fn instance_script_encounter_progress_helpers_match_cpp() {
    let mut script = InstanceScriptBase::new(4, 2);

    assert!(!script.is_encounter_in_progress_like_cpp());
    script.set_boss_state_like_cpp(1, EncounterState::InProgress);
    assert!(script.is_encounter_in_progress_like_cpp());
    script.set_boss_state_like_cpp(1, EncounterState::Done);
    assert!(!script.is_encounter_in_progress_like_cpp());
}

#[test]
fn instance_script_encounter_completed_by_dungeon_encounter_id_matches_cpp() {
    let store = DungeonEncounterStore::from_entries([
        encounter_with_bit(10, 4, 1),
        encounter_with_bit(20, 4, 2),
    ]);
    let mut script = InstanceScriptBase::new(4, 2);
    script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);
    script.load_dungeon_encounter_data(&store, 1, [20, 0, 0, 0]);

    assert!(!script.is_encounter_completed_like_cpp(&store, 10));
    script.set_boss_state_like_cpp(0, EncounterState::Done);
    assert!(script.is_encounter_completed_like_cpp(&store, 10));
    assert!(!script.is_encounter_completed_like_cpp(&store, 20));
    assert!(!script.is_encounter_completed_like_cpp(&store, 99));
}

#[test]
fn instance_script_encounter_completed_mask_by_boss_id_matches_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
    let mut script = InstanceScriptBase::new(4, 1);
    script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

    script.set_boss_state_like_cpp(0, EncounterState::InProgress);
    assert!(!script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 3, 0));

    script.set_boss_state_like_cpp(0, EncounterState::Done);
    assert!(script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 3, 0));
    assert!(!script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 2, 0));
    assert!(!script.is_encounter_completed_in_mask_by_boss_id_like_cpp(&store, 1 << 3, 99));
}

#[test]
fn set_boss_state_loading_initializes_without_effects_like_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
    let mut script = InstanceScriptBase::new(4, 1);
    script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

    let plan = script.set_boss_state_planned_like_cpp(&store, 0, EncounterState::NotStarted, false);

    assert!(plan.is_none());
    assert_eq!(script.boss_state(0), EncounterState::NotStarted);
}

#[test]
fn set_boss_state_in_progress_plans_cpp_start_effects() {
    let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
    let mut script = InstanceScriptBase::new(4, 1);
    script.create_like_cpp();
    script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

    let plan = script
        .set_boss_state_planned_like_cpp(&store, 0, EncounterState::InProgress, false)
        .unwrap();

    assert_eq!(plan.previous_state, EncounterState::NotStarted);
    assert_eq!(plan.new_state, EncounterState::InProgress);
    assert!(plan.initialize_combat_resurrections);
    assert!(plan.send_encounter_start);
    assert!(plan.notify_players_start);
    assert!(!plan.reset_combat_resurrections);
    assert!(!plan.send_encounter_end);
    assert_eq!(plan.dungeon_encounter_id, None);
    assert!(plan.update_doors_minions_and_spawn_groups);
}

#[test]
fn set_boss_state_done_plans_cpp_completion_effects() {
    let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
    let mut script = InstanceScriptBase::new(4, 1);
    script.create_like_cpp();
    script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

    let plan = script
        .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Done, false)
        .unwrap();

    assert_eq!(script.boss_state(0), EncounterState::Done);
    assert!(plan.reset_combat_resurrections);
    assert!(plan.send_encounter_end);
    assert!(plan.notify_players_end);
    assert_eq!(plan.dungeon_encounter_id, Some(10));
    assert!(plan.update_lock);
    assert!(plan.update_criteria);
    assert!(plan.send_boss_kill_credit);
    assert!(plan.update_lfg);
    assert!(plan.update_doors_minions_and_spawn_groups);
}

#[test]
fn set_boss_state_blocks_cpp_invalid_transitions() {
    let store = DungeonEncounterStore::from_entries([encounter_with_bit(10, 4, 3)]);
    let mut script = InstanceScriptBase::new(4, 1);
    script.create_like_cpp();
    script.load_dungeon_encounter_data(&store, 0, [10, 0, 0, 0]);

    assert!(
        script
            .set_boss_state_planned_like_cpp(&store, 99, EncounterState::Done, false)
            .is_none()
    );
    assert!(
        script
            .set_boss_state_planned_like_cpp(&store, 0, EncounterState::NotStarted, false)
            .is_none()
    );
    assert!(
        script
            .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Done, true)
            .is_none()
    );
    assert_eq!(script.boss_state(0), EncounterState::NotStarted);

    script
        .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Done, false)
        .unwrap();
    assert!(
        script
            .set_boss_state_planned_like_cpp(&store, 0, EncounterState::Fail, false)
            .is_none()
    );
    assert_eq!(script.boss_state(0), EncounterState::Done);
}

#[test]
fn combat_resurrection_interval_matches_cpp_player_count_rule() {
    assert_eq!(combat_resurrection_charge_interval_like_cpp(0), 0);
    assert_eq!(combat_resurrection_charge_interval_like_cpp(1), 5_400_000);
    assert_eq!(combat_resurrection_charge_interval_like_cpp(9), 600_000);
}

#[test]
fn combat_resurrection_initialize_update_and_gain_charge_match_cpp() {
    let mut tracker = CombatResurrectionTracker::default();

    tracker.initialize_like_cpp(1, 600_000);
    assert_eq!(tracker.charges(), 1);
    assert_eq!(tracker.timer_ms(), 600_000);
    assert!(tracker.timer_started());

    assert_eq!(tracker.update_like_cpp(100_000, 9), None);
    assert_eq!(tracker.timer_ms(), 500_000);

    assert_eq!(
        tracker.update_like_cpp(500_000, 9),
        Some(CombatResurrectionEvent::GainCharge {
            in_combat_res_count: 2,
            combat_res_charge_recovery: 600_000,
        })
    );
    assert_eq!(tracker.charges(), 2);
    assert_eq!(tracker.timer_ms(), 600_000);
}

#[test]
fn combat_resurrection_use_and_reset_match_cpp() {
    let mut tracker = CombatResurrectionTracker::default();
    tracker.initialize_like_cpp(1, 600_000);

    assert_eq!(
        tracker.use_charge_like_cpp(),
        CombatResurrectionEvent::InCombatResurrection
    );
    assert_eq!(tracker.charges(), 0);

    tracker.reset_like_cpp();
    assert_eq!(tracker.charges(), 0);
    assert_eq!(tracker.timer_ms(), 0);
    assert!(!tracker.timer_started());
    assert_eq!(tracker.update_like_cpp(600_000, 9), None);
}

#[test]
fn instance_script_combat_resurrection_wrappers_use_tracker_like_cpp() {
    let mut script = InstanceScriptBase::new(4, 1);

    script.initialize_combat_resurrections_like_cpp(1, 600_000);
    assert_eq!(script.combat_resurrections().charges(), 1);
    assert_eq!(
        script.update_combat_resurrection_like_cpp(600_000, 9),
        Some(CombatResurrectionEvent::GainCharge {
            in_combat_res_count: 2,
            combat_res_charge_recovery: 600_000,
        })
    );
    assert_eq!(
        script.use_combat_resurrection_like_cpp(),
        CombatResurrectionEvent::InCombatResurrection
    );
    assert_eq!(script.combat_resurrections().charges(), 1);

    script.reset_combat_resurrections_like_cpp();
    assert_eq!(
        script.combat_resurrections(),
        CombatResurrectionTracker::default()
    );
}

#[test]
fn entrance_location_prefers_temporary_and_set_clears_temporary_like_cpp() {
    let mut script = InstanceScriptBase::new(4, 1);

    assert_eq!(script.entrance_location_like_cpp(), 0);
    script.set_entrance_location_like_cpp(100);
    assert_eq!(script.entrance_location_like_cpp(), 100);

    script.set_temporary_entrance_location_like_cpp(200);
    assert_eq!(script.entrance_location_like_cpp(), 200);

    script.set_entrance_location_like_cpp(300);
    assert_eq!(script.entrance_location_like_cpp(), 300);
}

#[test]
fn entrance_location_for_completed_encounters_matches_cpp_base_behavior() {
    let mut script = InstanceScriptBase::new(4, 1);
    script.set_entrance_location_like_cpp(100);

    assert_eq!(
        script.entrance_location_for_completed_encounters_like_cpp(false, 0xFF),
        Some(100)
    );
    assert_eq!(
        script.entrance_location_for_completed_encounters_like_cpp(true, 0xFF),
        None
    );
}

#[test]
fn area_trigger_done_set_matches_cpp_mark_reset_query() {
    let mut script = InstanceScriptBase::new(4, 1);

    assert!(!script.is_area_trigger_done_like_cpp(7));
    script.mark_area_trigger_done_like_cpp(7);
    script.mark_area_trigger_done_like_cpp(7);
    assert!(script.is_area_trigger_done_like_cpp(7));

    script.reset_area_trigger_done_like_cpp(7);
    assert!(!script.is_area_trigger_done_like_cpp(7));
}

#[test]
fn boss_info_selects_first_any_or_matching_difficulty_like_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter(1, 0), encounter(2, 4)]);
    let mut script = InstanceScriptBase::new(4, 1);

    script.load_dungeon_encounter_data(&store, 0, [1, 2, 0, 0]);

    assert_eq!(script.boss_dungeon_encounter(&store, 0).unwrap().id, 1);
}

#[test]
fn boss_info_skips_non_matching_difficulty_like_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter(1, 3), encounter(2, 4)]);
    let mut script = InstanceScriptBase::new(4, 1);

    script.load_dungeon_encounter_data(&store, 0, [1, 2, 0, 0]);

    assert_eq!(script.boss_dungeon_encounter(&store, 0).unwrap().id, 2);
}

#[test]
fn load_dungeon_encounter_data_ignores_invalid_boss_or_missing_rows_like_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter(2, 4)]);
    let mut script = InstanceScriptBase::new(4, 1);

    script.load_dungeon_encounter_data(&store, 99, [2, 0, 0, 0]);
    assert!(script.boss_dungeon_encounter(&store, 0).is_none());

    script.load_dungeon_encounter_data(&store, 0, [1, 0, 0, 0]);
    assert!(script.boss_dungeon_encounter(&store, 0).is_none());
}

#[test]
fn creature_overload_uses_boss_ai_boss_id_like_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter(2, 4)]);
    let mut script = InstanceScriptBase::new(4, 2);
    let boss_ai = BossAiRef::new(1);

    script.load_dungeon_encounter_data(&store, 1, [2, 0, 0, 0]);

    assert_eq!(
        script
            .boss_dungeon_encounter_for_boss_ai(&store, Some(&boss_ai))
            .unwrap()
            .id,
        2
    );
}

#[test]
fn creature_overload_returns_none_when_dynamic_cast_fails_like_cpp() {
    let store = DungeonEncounterStore::from_entries([encounter(2, 4)]);
    let mut script = InstanceScriptBase::new(4, 2);

    script.load_dungeon_encounter_data(&store, 1, [2, 0, 0, 0]);

    assert!(
        script
            .boss_dungeon_encounter_for_boss_ai::<BossAiRef>(&store, None)
            .is_none()
    );
}
