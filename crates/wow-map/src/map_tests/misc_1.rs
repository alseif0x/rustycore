//! Misc scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn dynamic_tree_update_empty_tree_returns_before_timer_or_balance_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.mark_dynamic_tree_unbalanced_for_tests_like_cpp(3);

    let summary = map.update_dynamic_tree_like_cpp(250);

    assert_eq!(summary.diff_ms, 250);
    assert!(summary.empty);
    assert_eq!(summary.timer_before_ms, 200);
    assert_eq!(summary.timer_after_ms, 200);
    assert!(!summary.timer_passed);
    assert_eq!(summary.timer_reset_to_ms, None);
    assert_eq!(summary.unbalanced_before, 3);
    assert!(!summary.balanced);
    assert_eq!(summary.unbalanced_after, 3);

    let next = map.update_dynamic_tree_like_cpp(50);
    assert_eq!(next.timer_before_ms, 200);
    assert_eq!(next.unbalanced_before, 3);
}
#[test]
fn dynamic_tree_update_non_empty_clean_tree_consumes_timer_and_resets_without_balance_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.insert_gameobject_model_like_cpp(dynamic_model_key(1));
    map.mark_dynamic_tree_unbalanced_for_tests_like_cpp(0);

    let first = map.update_dynamic_tree_like_cpp(50);
    assert!(!first.empty);
    assert_eq!(first.timer_before_ms, 200);
    assert_eq!(first.timer_after_ms, 150);
    assert!(!first.timer_passed);
    assert_eq!(first.timer_reset_to_ms, None);
    assert_eq!(first.unbalanced_before, 0);
    assert!(!first.balanced);
    assert_eq!(first.unbalanced_after, 0);

    let second = map.update_dynamic_tree_like_cpp(150);
    assert_eq!(second.timer_before_ms, 150);
    assert_eq!(second.timer_after_ms, 200);
    assert!(second.timer_passed);
    assert_eq!(second.timer_reset_to_ms, Some(200));
    assert_eq!(second.unbalanced_before, 0);
    assert!(!second.balanced);
    assert_eq!(second.unbalanced_after, 0);
}
#[test]
fn dynamic_tree_update_non_empty_unbalanced_tree_balances_when_timer_passes_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.insert_gameobject_model_like_cpp(dynamic_model_key(1));
    map.insert_gameobject_model_like_cpp(dynamic_model_key(2));

    let first = map.update_dynamic_tree_like_cpp(199);
    assert_eq!(first.timer_before_ms, 200);
    assert_eq!(first.timer_after_ms, 1);
    assert!(!first.timer_passed);
    assert_eq!(first.unbalanced_before, 2);
    assert!(!first.balanced);
    assert_eq!(first.unbalanced_after, 2);

    let second = map.update_dynamic_tree_like_cpp(1);
    assert_eq!(second.timer_before_ms, 1);
    assert_eq!(second.timer_after_ms, 200);
    assert!(second.timer_passed);
    assert_eq!(second.timer_reset_to_ms, Some(200));
    assert_eq!(second.unbalanced_before, 2);
    assert!(second.balanced);
    assert_eq!(second.unbalanced_after, 0);

    let third = map.update_dynamic_tree_like_cpp(200);
    assert_eq!(third.unbalanced_before, 0);
    assert!(!third.balanced);
    assert_eq!(third.unbalanced_after, 0);
}
#[test]
fn dynamic_tree_insert_first_model_makes_tree_non_empty_and_update_consumes_timer_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45001);

    let inserted = map.insert_gameobject_model_like_cpp(key);
    assert_eq!(
        inserted.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(inserted.model_count_before, 0);
    assert_eq!(inserted.model_count_after, 1);
    assert_eq!(inserted.unbalanced_before, 0);
    assert_eq!(inserted.unbalanced_after, 1);
    assert!(map.contains_gameobject_model_like_cpp(key));

    let summary = map.update_dynamic_tree_like_cpp(50);
    assert!(!summary.empty);
    assert_eq!(summary.timer_before_ms, 200);
    assert_eq!(summary.timer_after_ms, 150);
    assert!(!summary.timer_passed);
    assert_eq!(summary.unbalanced_before, 1);
    assert_eq!(summary.unbalanced_after, 1);
}
#[test]
fn dynamic_tree_duplicate_insert_does_not_double_count_or_increment_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45002);

    let first = map.insert_gameobject_model_like_cpp(key);
    let duplicate = map.insert_gameobject_model_like_cpp(key);

    assert_eq!(
        first.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Inserted
    );
    assert_eq!(
        duplicate.status,
        DynamicMapTreeModelMutationStatusLikeCpp::AlreadyPresent
    );
    assert_eq!(duplicate.model_count_before, 1);
    assert_eq!(duplicate.model_count_after, 1);
    assert_eq!(duplicate.unbalanced_before, 1);
    assert_eq!(duplicate.unbalanced_after, 1);
}
#[test]
fn active_non_player_visit_sources_use_real_set_and_filter_stale_like_cpp() {
    let mut map = test_map();
    let mut active = test_gameobject_for_spawn(48505, 4850501);
    let active_guid = active.world().guid();
    active.world_mut().set_active(true);
    active.world_mut().object_mut().remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(active).unwrap())
        .unwrap();

    let mut stale = test_gameobject_for_spawn(48505, 4850502);
    let stale_guid = stale.world().guid();
    stale.world_mut().set_active(true);
    stale.world_mut().object_mut().remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(stale).unwrap())
        .unwrap();
    map.active_non_players_like_cpp.remove(&stale_guid);
    map.active_non_players_like_cpp
        .insert(guid(HighGuid::GameObject, 4850503));

    let active_sources = map.represented_active_non_player_sources_like_cpp();
    assert_eq!(active_sources, vec![active_guid]);
    let plan = map.map_update_visit_plan_like_cpp(
        std::iter::empty::<MapUpdatePlayerSources>(),
        active_sources,
        std::iter::empty::<ObjectGuid>(),
        1,
    );
    assert!(plan.process_relocation_notifies);
    assert_eq!(plan.nearby_visit_centers, vec![active_guid]);
}
#[test]
fn summon_object_wild_position_missing_dst_uses_default_player_radius_like_cpp() {
    let caster = Position::new(10.0, 20.0, 30.0, 0.0);

    let outcome = spell_effect_summon_object_wild_position_like_cpp(caster, 1.25, 0.75, None);

    assert_eq!(
        outcome.position,
        Position::new(
            10.0 + 1.25 + DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP,
            20.0,
            30.0,
            0.75
        )
    );
    assert!(!outcome.explicit_destination_used);
    assert!(outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(outcome.focus_object_orientation_represented);
    assert!(!outcome.collision_los_adjustment_represented);
}
#[test]
fn script_schedule_due_prefix_drains_sorted_and_keeps_future_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let delayed = map.schedule_represented_script_action_like_cpp(
        100,
        10,
        script_guid(1),
        script_guid(2),
        script_guid(3),
        1001,
    );
    let due_a = map.schedule_represented_script_action_like_cpp(
        100,
        5,
        script_guid(4),
        script_guid(5),
        script_guid(6),
        1002,
    );
    let due_b = map.schedule_represented_script_action_like_cpp(
        100,
        5,
        script_guid(7),
        script_guid(8),
        script_guid(9),
        1003,
    );

    assert_eq!(delayed.represented_increase_count, 1);
    assert_eq!(due_a.represented_increase_count, 1);
    assert_eq!(due_b.represented_increase_count, 1);
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 3);

    let summary = map.process_due_script_schedule_like_cpp(105);

    assert_eq!(summary.queued_before, 3);
    assert_eq!(summary.processed, 2);
    assert_eq!(summary.represented_decrease_count, 2);
    assert_eq!(summary.remaining, 1);
    assert!(!summary.empty_noop);
    assert_eq!(
        summary
            .processed_actions
            .iter()
            .map(|action| action.command_id)
            .collect::<Vec<_>>(),
        vec![1002, 1003]
    );
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 1);
    assert_eq!(
        map.represented_executed_script_actions_like_cpp()
            .iter()
            .map(|action| action.command_id)
            .collect::<Vec<_>>(),
        vec![1002, 1003]
    );

    let delayed_summary = map.process_script_schedule_update_order_like_cpp(110);
    assert_eq!(delayed_summary.processed_actions, vec![delayed.scheduled]);
    assert_eq!(delayed_summary.remaining, 0);
    assert!(delayed_summary.lock_entered);
}
#[test]
fn script_schedule_empty_update_order_is_noop_without_lock_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);

    let summary = map.process_script_schedule_update_order_like_cpp(100);

    assert!(summary.empty_noop);
    assert_eq!(summary.queued_before, 0);
    assert_eq!(summary.processed, 0);
    assert_eq!(summary.remaining, 0);
    assert!(!summary.lock_entered);
    assert!(!map.is_script_schedule_locked_like_cpp());
}
#[test]
fn script_schedule_zero_delay_processes_immediately_when_unlocked_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);

    let outcome = map.schedule_represented_script_action_like_cpp(
        200,
        0,
        script_guid(11),
        script_guid(12),
        script_guid(13),
        2001,
    );

    let immediate = outcome
        .immediate_process
        .expect("zero-delay represented schedule should process while unlocked");
    assert_eq!(immediate.queued_before, 1);
    assert_eq!(immediate.processed, 1);
    assert_eq!(immediate.remaining, 0);
    assert!(immediate.lock_entered);
    assert_eq!(immediate.processed_actions, vec![outcome.scheduled]);
    assert_eq!(outcome.remaining_after_schedule, 0);
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 0);
}
#[test]
fn script_schedule_zero_delay_remains_queued_when_locked_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.set_script_schedule_lock_for_test(true);

    let outcome = map.schedule_represented_script_action_like_cpp(
        300,
        0,
        script_guid(21),
        script_guid(22),
        script_guid(23),
        3001,
    );

    assert!(outcome.immediate_process.is_none());
    assert_eq!(outcome.remaining_after_schedule, 1);
    assert_eq!(map.represented_script_schedule_count_like_cpp(), 1);
    assert!(map.is_script_schedule_locked_like_cpp());
    map.set_script_schedule_lock_for_test(false);

    let summary = map.process_script_schedule_update_order_like_cpp(300);
    assert_eq!(summary.processed_actions, vec![outcome.scheduled]);
    assert_eq!(summary.remaining, 0);
}
#[test]
fn weather_timer_not_passed_does_not_call_default_weather_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44701);

    let summary = map.update_weather_like_cpp(999);

    assert_eq!(summary.interval_ms, 1_000);
    assert_eq!(summary.timer_current_before, 0);
    assert_eq!(summary.timer_current_after_update, 999);
    assert_eq!(summary.timer_current_after_reset, 999);
    assert!(!summary.timer_passed);
    assert_eq!(summary.zones_seen, 0);
    assert_eq!(summary.default_weather_updated, 0);
    assert_eq!(map.weather_update_timer_current_ms_like_cpp(), 999);
    assert_eq!(
        map.represented_zone_default_weather_update_diffs_like_cpp(44701),
        Some([].as_slice())
    );
}
#[test]
fn weather_timer_passed_exact_interval_calls_default_weather_with_interval_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44702);

    let summary = map.update_weather_like_cpp(1_000);

    assert!(summary.timer_passed);
    assert_eq!(summary.timer_current_before, 0);
    assert_eq!(summary.timer_current_after_update, 1_000);
    assert_eq!(summary.timer_current_after_reset, 0);
    assert_eq!(summary.zones_seen, 1);
    assert_eq!(summary.default_weather_updated, 1);
    assert_eq!(summary.default_weather_removed, 0);
    assert_eq!(summary.weather_update_call_diff_ms, Some(1_000));
    assert!(summary.script_update_regeneration_fanout_not_represented);
    assert_eq!(map.weather_update_timer_current_ms_like_cpp(), 0);
    assert_eq!(
        map.represented_zone_default_weather_update_diffs_like_cpp(44702),
        Some([1_000].as_slice())
    );
}
#[test]
fn weather_timer_overshoot_reset_preserves_modulo_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44703);

    let summary = map.update_weather_like_cpp(2_500);

    assert!(summary.timer_passed);
    assert_eq!(summary.timer_current_after_update, 2_500);
    assert_eq!(summary.timer_current_after_reset, 500);
    assert_eq!(map.weather_update_timer_current_ms_like_cpp(), 500);
    assert_eq!(summary.default_weather_updated, 1);
    assert_eq!(
        map.represented_zone_default_weather_update_diffs_like_cpp(44703),
        Some([1_000].as_slice())
    );
}
#[test]
fn weather_update_false_return_resets_default_weather_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.register_represented_zone_default_weather_for_test(44704);
    assert!(map.set_represented_zone_default_weather_next_update_alive_for_test(44704, false));

    let summary = map.update_weather_like_cpp(1_000);

    assert!(summary.timer_passed);
    assert_eq!(summary.default_weather_updated, 1);
    assert_eq!(summary.default_weather_removed, 1);
    assert!(
        map.represented_zone_dynamic_info_like_cpp(44704)
            .and_then(|zone| zone.default_weather.as_ref())
            .is_none()
    );
}
#[test]
fn guid_sequence_transport_can_be_set_for_future_global_sync_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.set_guid_sequence_like_cpp(HighGuid::Transport, 77),
        Ok(())
    );
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Transport), Ok(77));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Transport), Ok(78));
}
#[test]
fn urand_inclusive_like_cpp_stays_within_inclusive_bounds() {
    let mut map = test_map();
    map.seed_creature_level_rng_for_tests_like_cpp(0x407);

    let mut saw_min = false;
    let mut saw_max = false;
    for _ in 0..512 {
        let value = map.urand_inclusive_like_cpp(18, 20);
        assert!((18..=20).contains(&value));
        saw_min |= value == 18;
        saw_max |= value == 20;
    }

    assert!(saw_min, "inclusive C++ urand should be able to return min");
    assert!(saw_max, "inclusive C++ urand should be able to return max");
}
#[test]
#[should_panic(expected = "C++ urand requires max >= min")]
fn urand_inclusive_like_cpp_asserts_max_at_least_min_like_cpp() {
    let mut map = test_map();
    let _ = map.urand_inclusive_like_cpp(20, 18);
}
#[test]
fn game_event_npc_flag_live_consumer_applies_upper_bits_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(550, 55001, true)).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_event_npc_flags_by_spawn_id_like_cpp(550, 0xFFFF_FFFF_0000_0040);

    assert_eq!(outcome.live_creatures_mutated, 1);
    assert_eq!(outcome.npc_flags_low_applied, 1);
    assert_eq!(outcome.npc_flags2_applied, 1);
    let guid = map.creature_spawn_id_store_guids_like_cpp(550)[0];
    let creature = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(creature.ai_ownership().npc_flags, 0x40);
    assert_eq!(creature.ai_ownership().npc_flags2, 0xFFFF_FFFF);
    assert_eq!(creature.unit().data().npc_flags, [0x40, 0xFFFF_FFFF]);
}
#[test]
fn typed_player_counts_exclude_game_masters_like_cpp() {
    let mut map = test_map();
    let normal_guid = guid(HighGuid::Player, 42);
    let gm_guid = guid(HighGuid::Player, 43);

    let mut normal = Player::new(Some(7), false);
    normal
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(normal_guid);
    normal.unit_mut().world_mut().set_map(571, 7).unwrap();
    normal.unit_mut().world_mut().object_mut().add_to_world();
    map.insert_map_object_record(MapObjectRecord::new_player(normal).unwrap())
        .unwrap();

    let mut gm = Player::new(Some(8), false);
    gm.unit_mut().world_mut().object_mut().create(gm_guid);
    gm.unit_mut().world_mut().set_map(571, 7).unwrap();
    gm.unit_mut().world_mut().object_mut().add_to_world();
    gm.set_game_master_like_cpp(true);
    map.insert_map_object_record(MapObjectRecord::new_player(gm).unwrap())
        .unwrap();

    assert_eq!(map.typed_player_counts_like_cpp(), (2, 1));
}
#[test]
fn closure_scoped_entity_queries_return_owned_values_and_reject_wrong_kinds() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(506, 506, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let snapshot = map
        .with_creature_or_pet_like_cpp(creature_guid, |creature, owner_guid| {
            (
                creature.guid(),
                creature.current_health(),
                creature.position(),
                owner_guid,
            )
        })
        .expect("the canonical creature should be readable inside the storage scope");
    assert_eq!(snapshot.0, creature_guid);
    assert_eq!(snapshot.1, 100);
    assert_eq!(snapshot.2, Position::xyz(1.0, 2.0, 3.0));
    assert_eq!(snapshot.3, None);
    assert!(map.with_pet_like_cpp(creature_guid, |_| ()).is_none());
    assert!(
        map.with_world_object_by_kinds_like_cpp(
            creature_guid,
            &[AccessorObjectKind::GameObject],
            |_| (),
        )
        .is_none()
    );
    assert!(
        map.with_creature_or_pet_like_cpp(guid(HighGuid::Creature, 999_999), |_, _| ())
            .is_none()
    );
}
#[test]
fn send_object_updates_processes_dynamic_object_data_update_like_cpp() {
    use wow_entities::{DYNAMIC_OBJECT_DATA_PARENT_BIT, DYNAMIC_OBJECT_DATA_RADIUS_BIT};

    let mut map = test_map();
    let dynamic_object = test_dynamic_object_for_viewpoint(501001);
    let dynamic_object_guid = dynamic_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let record = map.entity_world.get_mut(&dynamic_object_guid).unwrap();
    assert!(!record.object().object().is_object_updated());
    record.dynamic_object_mut().unwrap().set_radius(12.5);
    assert!(record.object().object().is_object_updated());
    assert!(
        record
            .dynamic_object()
            .unwrap()
            .dynamic_object_data_changes_mask()
            .is_any_set()
    );

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(summary.queued_before, 1);
    assert_eq!(summary.processed, 1);
    assert_eq!(summary.cleared_update_masks, 1);
    assert_eq!(summary.skipped_not_in_world, 0);
    assert_eq!(summary.missing_or_stale, 0);
    assert_eq!(summary.fanout_not_represented, 1);
    assert_eq!(summary.dynamic_object_values_updates.len(), 1);
    let represented_values = &summary.dynamic_object_values_updates[0];
    assert_eq!(represented_values.guid, dynamic_object_guid);
    let dynamic_object_data = represented_values
        .values_update
        .dynamic_object_data
        .as_ref()
        .unwrap();
    assert!(
        dynamic_object_data
            .mask
            .is_set(DYNAMIC_OBJECT_DATA_PARENT_BIT)
    );
    assert!(
        dynamic_object_data
            .mask
            .is_set(DYNAMIC_OBJECT_DATA_RADIUS_BIT)
    );
    assert_eq!(dynamic_object_data.values.radius, 12.5);
    assert!(
        !map.map_object_record(dynamic_object_guid)
            .unwrap()
            .object()
            .object()
            .is_object_updated()
    );
    assert!(
        !map.map_object_record(dynamic_object_guid)
            .unwrap()
            .dynamic_object()
            .unwrap()
            .dynamic_object_data_changes_mask()
            .is_any_set()
    );
    assert!(
        !map.map_object_record(dynamic_object_guid)
            .unwrap()
            .dynamic_object()
            .unwrap()
            .values_update()
            .has_data()
    );
}
#[test]
fn area_trigger_update_decrements_duration_without_queue_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_update(4340101, 1_000, true);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 250);

    assert_eq!(outcome.status, AreaTriggerUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(750));
    assert_eq!(outcome.time_since_created_before_ms, Some(0));
    assert_eq!(outcome.time_since_created_after_ms, Some(250));
    assert!(outcome.non_static_movement_would_run);
    assert!(outcome.ai_update_would_run);
    assert!(outcome.target_list_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let area_trigger = map
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert_eq!(area_trigger.duration_ms(), 750);
    assert_eq!(area_trigger.time_since_created_ms(), 250);
    assert!(!area_trigger.is_removed());
}
#[test]
fn area_trigger_update_permanent_duration_increments_time_without_queue_like_cpp() {
    let mut map = test_map();
    let mut area_trigger = test_area_trigger_for_update(4340301, -1, true);
    area_trigger.set_spawn_id(4340301);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 1_000);

    assert_eq!(outcome.status, AreaTriggerUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(-1));
    assert_eq!(outcome.duration_after_ms, Some(-1));
    assert_eq!(outcome.time_since_created_after_ms, Some(1_000));
    assert!(!outcome.non_static_movement_would_run);
    assert!(outcome.ai_update_would_run);
    assert!(outcome.target_list_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn area_trigger_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_update(4340401, 500, false);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 250);

    assert_eq!(outcome.status, AreaTriggerUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.duration_before_ms, Some(500));
    assert_eq!(outcome.duration_after_ms, Some(500));
    assert_eq!(outcome.time_since_created_before_ms, Some(0));
    assert_eq!(outcome.time_since_created_after_ms, Some(0));
    assert!(!outcome.non_static_movement_would_run);
    assert!(!outcome.ai_update_would_run);
    assert!(!outcome.target_list_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let area_trigger = map
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert_eq!(area_trigger.duration_ms(), 500);
    assert_eq!(area_trigger.time_since_created_ms(), 0);
    assert!(!area_trigger.is_removed());
}
#[test]
fn area_trigger_update_missing_or_non_area_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::AreaTrigger, 4340501);
    let creature_guid = guid(HighGuid::Creature, 4340502);
    let creature = test_creature_for_spawn(43405, 4340502, true);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let missing = map.update_area_trigger_like_cpp(missing_guid, 250);
    let non_area = map.update_area_trigger_like_cpp(creature_guid, 250);

    assert_eq!(
        missing.status,
        AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger
    );
    assert_eq!(
        non_area.status,
        AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
}
#[test]
fn send_object_updates_clears_in_world_changed_object_like_cpp() {
    let mut map = test_map();
    let game_object = test_gameobject_for_spawn(4450101, 4450101);
    let game_object_guid = game_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();
    map.entity_world
        .get_mut(&game_object_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .set_scale(2.0);

    let before = map.map_object(game_object_guid).unwrap().object();
    assert!(before.is_object_updated());
    assert!(!before.changed_fields().is_empty());

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(
        summary,
        SendObjectUpdatesSummaryLikeCpp {
            queued_before: 1,
            processed: 1,
            cleared_update_masks: 1,
            skipped_not_in_world: 0,
            missing_or_stale: 0,
            fanout_not_represented: 1,
            dynamic_object_values_updates: Vec::new(),
            player_values_updates: Vec::new(),
            unit_values_updates: Vec::new(),
        }
    );
    let after = map.map_object(game_object_guid).unwrap().object();
    assert!(!after.is_object_updated());
    assert!(after.changed_fields().is_empty());
}
#[test]
fn send_object_updates_consumes_queued_player_stand_state_like_cpp() {
    let mut map = test_map();
    let player_guid = ObjectGuid::create_player(1, 4_450_151);
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.clear_data_changes();

    player.set_inebriation_like_cpp(7);
    player.set_money(42);
    player
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);
    assert!(player.unit().world().object().is_object_updated());
    assert!(
        player
            .unit()
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_STAND_STATE_BIT)
    );
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(summary.queued_before, 1);
    assert_eq!(summary.processed, 1);
    assert_eq!(summary.cleared_update_masks, 1);
    assert_eq!(summary.player_values_updates.len(), 1);
    let captured = &summary.player_values_updates[0];
    assert_eq!(captured.guid, player_guid);
    assert!(
        captured
            .values_update
            .unit_data
            .as_ref()
            .is_some_and(|data| data.mask.is_set(UNIT_DATA_STAND_STATE_BIT))
    );
    assert!(
        captured
            .values_update
            .player_data
            .as_ref()
            .is_some_and(|data| data.mask.is_set(PLAYER_DATA_INEBRIATION_BIT)),
        "an unrelated PlayerData delta is captured before masks are cleared"
    );
    assert!(
        captured
            .values_update
            .active_player_data
            .as_ref()
            .is_some_and(|data| data.mask.is_set(ACTIVE_PLAYER_DATA_COINAGE_BIT)),
        "an unrelated ActivePlayerData delta is captured before masks are cleared"
    );
    let player = map
        .map_object_record(player_guid)
        .and_then(MapObjectRecord::player)
        .expect("typed Player remains on map");
    assert_eq!(
        player.unit().stand_state_like_cpp(),
        UnitStandStateType::Sit
    );
    assert!(!player.unit().world().object().is_object_updated());
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
    assert!(
        !player
            .unit()
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_STAND_STATE_BIT),
        "canonical SendObjectUpdates consumes the Player UnitData delta"
    );
}
#[test]
fn send_object_updates_skips_unchanged_objects_like_cpp() {
    let mut map = test_map();
    let game_object = test_gameobject_for_spawn(4450201, 4450201);
    let game_object_guid = game_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();

    let before = map.map_object(game_object_guid).unwrap().object();
    assert!(!before.is_object_updated());
    assert!(before.changed_fields().is_empty());

    let summary = map.send_object_updates_like_cpp();

    assert_eq!(summary, SendObjectUpdatesSummaryLikeCpp::default());
    let after = map.map_object(game_object_guid).unwrap().object();
    assert!(!after.is_object_updated());
    assert!(after.changed_fields().is_empty());
}
#[test]
fn send_object_updates_not_in_world_updated_state_is_not_publicly_constructible() {
    // C++ `_updateObjects` should never contain not-in-world objects:
    // `Object::AddToObjectUpdateIfNeeded` only sets `m_objectUpdated` when
    // `m_inWorld`, and `Object::remove_from_world`/`ClearUpdateMask(true)`
    // clears the flag. Rust mirrors that public invariant in
    // `EntityObject`, whose `object_updated`/`in_world` fields are private to
    // `wow-entities`, so wow-map tests cannot construct the defensive
    // `skipped_not_in_world` branch without unsafe/private-field hacks.
    let mut map = test_map();
    let mut game_object = test_gameobject_for_spawn(4450301, 4450301);
    game_object.world_mut().object_mut().set_scale(2.0);
    game_object.world_mut().object_mut().remove_from_world();
    let game_object_guid = game_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();

    assert!(
        !map.map_object(game_object_guid)
            .unwrap()
            .object()
            .is_in_world()
    );
    assert!(
        !map.map_object(game_object_guid)
            .unwrap()
            .object()
            .is_object_updated()
    );
    assert_eq!(
        map.send_object_updates_like_cpp(),
        SendObjectUpdatesSummaryLikeCpp::default()
    );
}
#[test]
fn transport_update_mutates_typed_canonical_record_like_cpp() {
    let mut map = test_map();
    let transport = test_transport_for_update(4390101, true);
    let transport_guid = transport.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();

    let outcome = map.update_transport_like_cpp(transport_guid, 50, 10_000);

    assert_eq!(outcome.status, TransportUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.path_progress_before_ms, Some(100));
    assert_eq!(outcome.path_progress_after_ms, Some(150));
    assert_eq!(outcome.timer_ms, Some(150));
    assert!(outcome.position_update_represented);
    let transport = map
        .map_object_record(transport_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 150);
}
