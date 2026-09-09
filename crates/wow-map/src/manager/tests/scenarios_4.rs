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
