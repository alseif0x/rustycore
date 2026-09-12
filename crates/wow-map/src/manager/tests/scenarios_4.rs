//! Managed-map lifecycle and updater regression scenarios, part 4 of 4.
//!
//! Moved out of the manager.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn create_map_decision_world_uses_zero_or_team_instance_like_cpp() {
    let mut manager = MapManager::default();
    let mut split_entry = world_entry(530);
    split_entry.split_by_faction = true;

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(world_entry(0)),
            Some(player()),
            |_, _| None,
            None,
            |_, _| None,
        ),
        CreateMapDecision::Create {
            key: MapKey::new(0, 0),
            difficulty_id: 0,
            kind: ManagedMapKind::World,
            side_effects: Vec::new(),
        }
    );
    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(split_entry),
            Some(player()),
            |_, _| None,
            None,
            |_, _| None,
        ),
        CreateMapDecision::Create {
            key: MapKey::new(530, 469),
            difficulty_id: 0,
            kind: ManagedMapKind::World,
            side_effects: Vec::new(),
        }
    );
}

#[test]
fn create_map_decision_battleground_requires_instance_and_bg_pointer_like_cpp() {
    let mut manager = MapManager::default();
    let entry = CreateMapEntryContext {
        map_id: 489,
        kind: CreateMapEntryKind::BattlegroundOrArena,
        split_by_faction: false,
        flex_locking: false,
    };

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(entry),
            Some(player()),
            |_, _| None,
            None,
            |_, _| None,
        ),
        CreateMapDecision::Reject {
            side_effects: Vec::new(),
        }
    );

    let mut bg_player = player();
    bg_player.battleground_id = 12;
    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(entry),
            Some(bg_player),
            |_, _| None,
            None,
            |_, _| None,
        ),
        CreateMapDecision::Reject {
            side_effects: vec![CreateMapSideEffect::TeleportToBattlegroundEntryPoint],
        }
    );

    bg_player.has_battleground = true;
    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(entry),
            Some(bg_player),
            |_, _| None,
            None,
            |_, _| None,
        ),
        CreateMapDecision::Create {
            key: MapKey::new(489, 12),
            difficulty_id: 0,
            kind: ManagedMapKind::Battleground,
            side_effects: Vec::new(),
        }
    );
}

#[test]
fn create_map_decision_dungeon_uses_active_lock_and_resets_difficulty_like_cpp() {
    let mut manager = MapManager::default();
    manager
        .create_map_entry(
            33,
            42,
            2,
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
        )
        .set_instance_lock_token(Some(9));

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(dungeon_entry(33, false)),
            Some(player()),
            |_, requested| Some(difficulty(requested, true, true)),
            Some(CreateMapInstanceLockContext {
                instance_id: 42,
                difficulty_id: 2,
                token: 9,
                owner_guid_counter: 77,
            }),
            |_, _| None,
        ),
        CreateMapDecision::Existing {
            key: MapKey::new(33, 42),
            difficulty_id: 2,
            side_effects: Vec::new(),
        }
    );
}

#[test]
fn create_map_decision_normal_dungeon_reuses_recent_instance_like_cpp() {
    let mut manager = MapManager::default();
    let mut player = player();
    player.player_recent_instance_id = 7;

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(dungeon_entry(33, false)),
            Some(player),
            |_, requested| Some(difficulty(requested, false, true)),
            None,
            |_, _| None,
        ),
        CreateMapDecision::Create {
            key: MapKey::new(33, 7),
            difficulty_id: 1,
            kind: ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
            side_effects: vec![CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 7 }],
        }
    );
}

#[test]
fn create_map_decision_dungeon_generates_instance_and_lock_side_effect_like_cpp() {
    let mut manager = MapManager::default();
    manager.init_instance_ids(3);
    manager.register_instance_id(1);

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(dungeon_entry(631, false)),
            Some(player()),
            |_, requested| Some(difficulty(requested, true, true)),
            None,
            |_, _| None,
        ),
        CreateMapDecision::Create {
            key: MapKey::new(631, 2),
            difficulty_id: 1,
            kind: ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            side_effects: vec![
                CreateMapSideEffect::CreateInstanceLockForNewInstance {
                    owner_guid_counter: 77,
                    instance_id: 2,
                },
                CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 2 },
            ],
        }
    );
}

#[test]
fn create_map_decision_flex_lock_conflict_regenerates_instance_like_cpp() {
    let mut manager = MapManager::default();
    manager.init_instance_ids(50);
    for instance_id in 1..=42 {
        manager.register_instance_id(instance_id);
    }
    manager
        .create_map_entry(
            631,
            42,
            3,
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
        )
        .set_instance_lock_token(Some(100));

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(dungeon_entry(631, true)),
            Some(player()),
            |_, requested| Some(difficulty(requested, true, false)),
            Some(CreateMapInstanceLockContext {
                instance_id: 42,
                difficulty_id: 3,
                token: 200,
                owner_guid_counter: 77,
            }),
            |_, _| None,
        ),
        CreateMapDecision::Create {
            key: MapKey::new(631, 43),
            difficulty_id: 1,
            kind: ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            side_effects: vec![
                CreateMapSideEffect::SetInstanceLockInstanceId { instance_id: 43 },
                CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 43 },
            ],
        }
    );
}

#[test]
fn find_instance_id_for_player_matches_cpp_world_bg_and_garrison_branches() {
    let manager = MapManager::default();
    let mut split = world_entry(609);
    split.split_by_faction = true;
    let bg = CreateMapEntryContext {
        map_id: 489,
        kind: CreateMapEntryKind::BattlegroundOrArena,
        split_by_faction: false,
        flex_locking: false,
    };
    let garrison = CreateMapEntryContext {
        map_id: 1152,
        kind: CreateMapEntryKind::Garrison,
        split_by_faction: false,
        flex_locking: false,
    };
    let mut player = player();
    player.team_id = 1;
    player.battleground_id = 12;

    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(world_entry(0)),
            Some(player),
            |_, _| None,
            None,
            |_, _| None,
        ),
        0
    );
    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(split),
            Some(player),
            |_, _| None,
            None,
            |_, _| None,
        ),
        1
    );
    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(bg),
            Some(player),
            |_, _| None,
            None,
            |_, _| None,
        ),
        12
    );
    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(garrison),
            Some(player),
            |_, _| None,
            None,
            |_, _| None,
        ),
        77
    );
}

#[test]
fn find_instance_id_for_player_matches_cpp_dungeon_lock_and_recent_rules() {
    let mut manager = MapManager::default();
    manager
        .create_map_entry(
            631,
            42,
            3,
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
        )
        .set_instance_lock_token(Some(100));
    let mut player = player();
    player.player_recent_instance_id = 7;

    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(dungeon_entry(33, false)),
            Some(player),
            |_, requested| Some(difficulty(requested, false, true)),
            None,
            |_, _| None,
        ),
        7
    );
    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(dungeon_entry(631, true)),
            Some(player),
            |_, requested| Some(difficulty(requested, true, false)),
            Some(CreateMapInstanceLockContext {
                instance_id: 42,
                difficulty_id: 3,
                token: 200,
                owner_guid_counter: 77,
            }),
            |_, _| None,
        ),
        0
    );
    assert_eq!(
        manager.find_instance_id_for_player_like_cpp(
            Some(dungeon_entry(631, true)),
            Some(player),
            |_, requested| Some(difficulty(requested, true, false)),
            Some(CreateMapInstanceLockContext {
                instance_id: 42,
                difficulty_id: 3,
                token: 100,
                owner_guid_counter: 77,
            }),
            |_, _| None,
        ),
        42
    );
}

// Anchor: canonical motor B (map.rs:6286-6295) calls runtime_update_plan and
// stores actions_recorded but has NO match that dispatches AiUpdateTick/
// MeleeAttackIfReady — no real AI/combat/threat/fanout. Characterizes the split
// vs. legacy motor A (WorldSession::tick_creatures_sync / tick_combat_sync).
#[test]
fn canonical_map_update_visits_creature_with_no_real_ai_combat_effect_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 9990001, true);

    let position_before = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .map_object(creature_guid)
        .unwrap()
        .position();
    let health_before = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap()
        .unit()
        .data()
        .health;

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();

    let summary = managed_map.last_creatures_update_summary();
    assert!(
        summary.visited >= 1,
        "canonical tick must visit the creature"
    );
    assert!(
        summary.actions_recorded > 0,
        "canonical tick must record plan actions (runtime_update_plan was called)"
    );

    let position_after = managed_map
        .map()
        .map_object(creature_guid)
        .unwrap()
        .position();
    let health_after = managed_map
        .map()
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap()
        .unit()
        .data()
        .health;

    assert_eq!(
        position_after, position_before,
        "canonical tick must not change creature position — no real AI/combat executed"
    );
    assert_eq!(
        health_after, health_before,
        "canonical tick must not change creature health — no real AI/combat executed"
    );
}

// ---------------------------------------------------------------------------
// #787 — the split canonical tick and the map-reference membership order.
// ---------------------------------------------------------------------------

#[test]
fn split_tick_runs_the_dynamic_tree_phase_before_the_session_pass_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let map = manager.find_map_mut(1, 0).unwrap();
    map.map_mut()
        .set_dynamic_tree_model_count_for_tests_like_cpp(1);
    map.map_mut()
        .mark_dynamic_tree_unbalanced_for_tests_like_cpp(2);

    let plan = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");

    // C++ `Map::Update` runs `_dynamicTree.update(t_diff)` before the map's
    // world sessions (`Map.cpp:668-680`), so the tree has already advanced when
    // the coordinator releases its guards.
    let map = manager.find_map(1, 0).unwrap();
    let tree = map.last_dynamic_tree_update_summary_like_cpp();
    assert_eq!(tree.diff_ms, 199);
    assert!(!tree.empty);
    assert_eq!(map.update_calls(), [199]);
    assert_eq!(plan.effective_diff_ms(), 199);
    assert_eq!(plan.updated_maps_like_cpp().len(), 1);
    assert_eq!(plan.updated_maps_like_cpp()[0].key, MapKey::new(1, 0));
    assert!(plan.destroyed_maps_like_cpp().is_empty());
}

#[test]
fn a_tick_that_has_not_passed_the_timer_produces_no_plan_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1_000);
    manager.create_world_map(1, 0);

    assert!(manager.begin_tick_like_cpp(10).is_timer_not_passed());
}

#[test]
fn the_resumed_tick_uses_the_saved_diff_and_runs_the_remaining_phases_once() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);

    let plan = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    let effective = plan.effective_diff_ms();
    assert_eq!(
        manager.resume_tick_like_cpp(
            plan,
            None,
            None::<
                &mut fn(
                    &mut Map,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
        ),
        MapTickResumeLikeCpp::Resumed
    );

    let map = manager.find_map(1, 0).unwrap();
    // One dynamic-tree phase at the split and one delayed update at the resume,
    // for one tick: the split never doubles a phase.
    assert_eq!(map.update_calls(), [effective]);
    assert_eq!(map.delayed_update_calls(), [effective]);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
}

#[test]
fn the_one_shot_tick_and_the_split_tick_leave_the_same_phase_counts() {
    let mut split = MapManager::new(MIN_GRID_DELAY_MS, 1);
    split.create_world_map(1, 0);
    let plan = split
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    split.resume_tick_like_cpp(
        plan,
        None,
        None::<
            &mut fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >,
    );

    let mut one_shot = MapManager::new(MIN_GRID_DELAY_MS, 1);
    one_shot.create_world_map(1, 0);
    one_shot.update(199);

    assert_eq!(
        split.find_map(1, 0).unwrap().update_calls(),
        one_shot.find_map(1, 0).unwrap().update_calls()
    );
    assert_eq!(
        split.find_map(1, 0).unwrap().delayed_update_calls(),
        one_shot.find_map(1, 0).unwrap().delayed_update_calls()
    );
}

#[test]
fn a_map_that_disappeared_after_the_split_is_skipped_by_the_resume() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);

    let plan = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    let effective = plan.effective_diff_ms();
    assert_eq!(plan.updated_maps_like_cpp().len(), 2);

    // Between the split and the resumption the coordinator holds no guard, so
    // another owner can retire a map. The resumed tick must skip it instead of
    // resurrecting or panicking on it.
    assert!(manager.destroy_map(2, 0));
    manager.resume_tick_like_cpp(
        plan,
        None,
        None::<
            &mut fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >,
    );

    assert!(manager.find_map(2, 0).is_none());
    let survivor = manager.find_map(1, 0).unwrap();
    assert_eq!(survivor.delayed_update_calls(), [effective]);
}

#[test]
fn a_second_begin_while_a_tick_is_split_is_refused_without_advancing_the_timer() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);

    let plan = manager
        .begin_tick_like_cpp(200)
        .into_started()
        .expect("timer passed");
    let effective = plan.effective_diff_ms();

    // The coordinator holds no guard here, so another caller can reach the
    // manager. C++ has one `MapManager::Update` per world tick: the second
    // caller must neither start a tick nor consume this diff, or the pending
    // resumption would run with a current value it never admitted.
    let second = manager.begin_tick_like_cpp(200);
    assert!(second.is_busy());

    manager.resume_tick_like_cpp(
        plan,
        None,
        None::<
            &mut fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >,
    );

    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.update_calls(), [effective]);
    assert_eq!(map.delayed_update_calls(), [effective]);

    // The refused diff was not consumed, so the timer still needs a full
    // interval before the next tick is admitted.
    assert!(manager.begin_tick_like_cpp(199).is_timer_not_passed());
    assert!(manager.begin_tick_like_cpp(1).into_started().is_some());
}

#[test]
fn a_plan_whose_epoch_the_manager_no_longer_awaits_resumes_nothing() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);

    let plan = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    let epoch = plan.epoch_like_cpp();
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );

    // Abandoning returns the manager to Idle; the plan value that was consumed
    // there cannot be replayed, and a plan from any other tick is refused.
    assert_eq!(
        manager.abandon_tick_like_cpp(plan),
        MapTickResumeLikeCpp::Resumed
    );

    let later = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    assert_ne!(later.epoch_like_cpp(), epoch);
    manager.abandon_tick_like_cpp(later);

    let stale = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    let stale_epoch = stale.epoch_like_cpp();
    manager.abandon_tick_like_cpp(stale);
    let refused = manager.begin_tick_like_cpp(199).into_started();
    let refused = refused.expect("timer passed");
    let refused_epoch = refused.epoch_like_cpp();
    assert_ne!(refused_epoch, stale_epoch);

    // The map ran its dynamic-tree phase once per admitted tick and no delayed
    // update at all: an abandoned tick runs no remaining phase.
    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.delayed_update_calls().len(), 0);
    manager.abandon_tick_like_cpp(refused);
}

#[test]
fn a_map_recreated_under_the_same_key_during_the_pass_is_not_resumed_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);

    let plan = manager
        .begin_tick_like_cpp(199)
        .into_started()
        .expect("timer passed");
    let effective = plan.effective_diff_ms();

    // `MapKey` is reusable. The map admitted by this tick is destroyed and a new
    // one takes its key while no guard is held; the replacement never ran a
    // dynamic-tree phase, so it must not receive the phases that follow one.
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);

    manager.resume_tick_like_cpp(
        plan,
        None,
        None::<
            &mut fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >,
    );

    let replacement = manager.find_map(1, 0).unwrap();
    assert!(replacement.update_calls().is_empty());
    // `MapManager::Update` still runs `DelayedUpdate` over every map it holds at
    // that point (`MapManager.cpp:314-317`), the replacement included.
    assert_eq!(replacement.delayed_update_calls(), [effective]);
}
