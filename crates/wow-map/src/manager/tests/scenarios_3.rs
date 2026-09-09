//! Managed-map lifecycle and updater regression scenarios, part 3 of 4.
//!
//! Moved out of the manager.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn map_manager_update_drains_live_creature_gameobject_area_trigger_move_lists_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 4420101, true);
    let game_object_guid = insert_game_object_for_update(&mut manager, 4420102, 0, true);
    let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4420103, 100, true);
    let creature_position = Position::xyz(10.5, 20.5, 30.5);
    let game_object_position = Position::xyz(13.5, 23.5, 33.5);
    let area_trigger_position = Position::xyz(12.5, 22.5, 32.5);

    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        assert_eq!(
            map.add_creature_to_move_list_like_cpp(creature_guid, creature_position),
            crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
        );
        assert_eq!(
            map.add_game_object_to_move_list_like_cpp(game_object_guid, game_object_position),
            crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
        );
        assert_eq!(
            map.add_area_trigger_to_move_list_like_cpp(area_trigger_guid, area_trigger_position),
            crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
        );
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    let map = managed_map.map();
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        0
    );
    assert_eq!(
        map.map_object(creature_guid).unwrap().position(),
        creature_position
    );
    assert_eq!(
        map.map_object(game_object_guid).unwrap().position(),
        game_object_position
    );
    assert_eq!(
        map.map_object(area_trigger_guid).unwrap().position(),
        area_trigger_position
    );
    assert_eq!(
        managed_map.last_live_move_list_drain_summary_like_cpp(),
        LiveMoveListDrainSummaryLikeCpp {
            creature: MoveListDrainSummaryLikeCpp {
                family: Some(MapObjectMoveListFamilyLikeCpp::Creature),
                processed: 1,
                relocated: 1,
                ..Default::default()
            },
            game_object: MoveListDrainSummaryLikeCpp {
                family: Some(MapObjectMoveListFamilyLikeCpp::GameObject),
                processed: 1,
                relocated: 1,
                ..Default::default()
            },
            area_trigger: MoveListDrainSummaryLikeCpp {
                family: Some(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
                processed: 1,
                relocated: 1,
                ..Default::default()
            },
        }
    );
}

#[test]
fn map_manager_update_leaves_dynamic_object_move_list_queued_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4420201, 100, true);
    let original_position = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .map_object(dynamic_object_guid)
        .unwrap()
        .position();
    let queued_position = Position::xyz(11.5, 21.5, 31.5);

    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        assert_eq!(
            map.add_dynamic_object_to_move_list_like_cpp(dynamic_object_guid, queued_position),
            crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
        );
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    let map = managed_map.map();
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::DynamicObject),
        1
    );
    assert_eq!(
        map.map_object(dynamic_object_guid).unwrap().position(),
        original_position
    );
    assert_eq!(
        managed_map.last_live_move_list_drain_summary_like_cpp(),
        LiveMoveListDrainSummaryLikeCpp {
            creature: MoveListDrainSummaryLikeCpp {
                family: Some(MapObjectMoveListFamilyLikeCpp::Creature),
                ..Default::default()
            },
            game_object: MoveListDrainSummaryLikeCpp {
                family: Some(MapObjectMoveListFamilyLikeCpp::GameObject),
                ..Default::default()
            },
            area_trigger: MoveListDrainSummaryLikeCpp {
                family: Some(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
                ..Default::default()
            },
        }
    );
}

#[test]
fn map_manager_update_processes_live_relocation_notifies_for_player_source_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let player_guid =
        insert_player_for_relocation_notify(&mut manager, 4430101, Position::xyz(10.0, 20.0, 30.0));
    let creature_guid = insert_creature_at_for_relocation_notify(
        &mut manager,
        4430102,
        Position::xyz(10.5, 20.5, 30.5),
        true,
    );
    let player_normal_guid =
        insert_player_for_relocation_notify(&mut manager, 4440103, Position::xyz(11.0, 21.0, 31.0));
    let other_creature_guid = insert_creature_at_for_relocation_notify(
        &mut manager,
        4440104,
        Position::xyz(11.5, 21.5, 31.5),
        false,
    );

    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        map.get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        map.get_typed_creature_mut(creature_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        assert!(
            map.map_object(player_guid)
                .unwrap()
                .object()
                .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
        );
        assert!(test_object_needs_notify_visibility(map, creature_guid));
    }

    assert_eq!(
        manager.update(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32),
        Some(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32)
    );

    let managed_map = manager.find_map(1, 0).unwrap();
    let outcome = managed_map.last_process_relocation_notifies_outcome_like_cpp();
    assert_eq!(
        outcome.process_plan.diff_ms,
        DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32
    );
    assert!(!outcome.process_plan.delayed_relocation_cells.is_empty());
    assert!(!outcome.process_plan.reset_timer_grids.is_empty());
    assert!(
        outcome
            .delayed_plan
            .cell_plans
            .iter()
            .any(|cell| cell.plan.player_relocations.contains(&player_guid))
    );
    assert!(
        outcome
            .delayed_plan
            .cell_plans
            .iter()
            .any(|cell| cell.plan.creature_relocations.contains(&creature_guid))
    );
    let creature_visibility_plan = outcome
        .visibility_plans
        .creature_plans
        .iter()
        .find(|plan| plan.creature_guid == creature_guid)
        .expect("live DelayedUnitRelocation creature visibility plan");
    assert!(
        creature_visibility_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_normal_guid)
    );
    assert!(
        creature_visibility_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(creature_guid, player_guid))
    );
    assert!(
        creature_visibility_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(creature_guid, other_creature_guid))
    );
    assert!(
        creature_visibility_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(other_creature_guid, creature_guid))
    );
    assert!(
        outcome
            .visibility_plans
            .player_plans
            .iter()
            .any(|plan| plan.player_guid == player_guid && plan.viewpoint_guid == player_guid)
    );
    assert!(
        outcome
            .reset_outcome
            .reset_player_guids
            .contains(&player_guid)
    );
    assert!(
        outcome
            .reset_outcome
            .reset_creature_guids
            .contains(&creature_guid)
    );
    assert!(!test_object_needs_notify_visibility(
        managed_map.map(),
        player_guid
    ));
    assert!(!test_object_needs_notify_visibility(
        managed_map.map(),
        creature_guid
    ));
}

#[test]
fn map_manager_update_skips_process_relocation_notifies_without_sources_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_at_for_relocation_notify(
        &mut manager,
        4430201,
        Position::xyz(10.5, 20.5, 30.5),
        false,
    );

    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        map.get_typed_creature_mut(creature_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    assert_eq!(
        manager.update(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32),
        Some(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32)
    );

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map.last_process_relocation_notifies_outcome_like_cpp(),
        ProcessRelocationNotifiesOutcome::default()
    );
    assert!(test_object_needs_notify_visibility(
        managed_map.map(),
        creature_guid
    ));
}

#[test]
fn map_manager_update_process_relocation_notifies_uses_post_drain_position_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let moving_active_guid = insert_creature_at_for_relocation_notify(
        &mut manager,
        4430301,
        Position::xyz(10.0, 20.0, 30.0),
        true,
    );
    let notify_guid = insert_creature_at_for_relocation_notify(
        &mut manager,
        4430302,
        Position::xyz(1500.0, 1500.0, 30.0),
        false,
    );
    let post_drain_position = Position::xyz(1500.5, 1500.5, 30.5);

    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        map.get_typed_creature_mut(notify_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        assert_eq!(
            map.add_creature_to_move_list_like_cpp(moving_active_guid, post_drain_position),
            crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
        );
    }

    assert_eq!(
        manager.update(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32),
        Some(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32)
    );

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        managed_map
            .map()
            .map_object(moving_active_guid)
            .unwrap()
            .position(),
        post_drain_position
    );
    assert_eq!(
        managed_map
            .last_live_move_list_drain_summary_like_cpp()
            .creature
            .relocated,
        1
    );
    let outcome = managed_map.last_process_relocation_notifies_outcome_like_cpp();
    assert!(
        outcome
            .reset_outcome
            .reset_creature_guids
            .contains(&notify_guid)
    );
    assert!(!test_object_needs_notify_visibility(
        managed_map.map(),
        notify_guid
    ));
}

#[test]
fn create_map_decision_rejects_missing_player_or_map_entry_like_cpp() {
    let mut manager = MapManager::default();

    assert_eq!(
        manager.create_map_decision_like_cpp(
            Some(world_entry(1)),
            None,
            |_, _| None,
            None,
            |_, _| None,
        ),
        CreateMapDecision::Reject {
            side_effects: Vec::new(),
        }
    );
    assert_eq!(
        manager.create_map_decision_like_cpp(None, Some(player()), |_, _| None, None, |_, _| None),
        CreateMapDecision::Reject {
            side_effects: Vec::new(),
        }
    );
}
