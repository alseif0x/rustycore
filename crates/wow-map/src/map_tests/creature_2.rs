//! Creature scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn spawn_group_spawn_loaded_grid_loader_some_inserts_creature_like_cpp() {
    let group = spawn_group(3951, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::Creature,
            111,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, _object_type, spawn_id, _force| {
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(test_creature_for_spawn(spawn_id, 111, true))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 0);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(111), 1);
}
#[test]
fn linked_respawn_time_reads_creature_and_gameobject_timers_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1234,
        grid_id: 7,
    });
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 300,
        entry: 88,
        respawn_time: 5678,
        grid_id: 7,
    });
    let slave_creature = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master_creature = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let slave_go = linked_respawn_guid(HighGuid::GameObject, 43, 101);
    let master_go = linked_respawn_guid(HighGuid::GameObject, 88, 300);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(slave_creature, master_creature);
    linked.insert_like_cpp(slave_go, master_go);

    assert_eq!(
        map.get_linked_respawn_time_like_cpp(slave_creature, &linked),
        1234
    );
    assert_eq!(
        map.get_linked_respawn_time_like_cpp(slave_go, &linked),
        5678
    );
}
#[test]
fn process_respawns_composite_live_creature_blocker_deletes_due_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(67, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(100, 100, true)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 100, 10));

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.deleted_live_object_blocker, 1);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .is_none()
    );
}
#[test]
fn dynamic_respawn_creature_uses_creature_rate_and_minimum() {
    let context = dynamic_respawn_context(Some(SpawnObjectType::Creature));
    let scaled = apply_dynamic_mode_respawn_scaling_like_cpp(120, context);

    assert_eq!(scaled.delay_secs, 30);
    assert!(scaled.was_scaled());
}
#[test]
fn map_object_store_can_hold_typed_creature_entity_like_cpp() {
    let mut map = test_map();
    let mut creature = Creature::new(false);
    let guid = guid(HighGuid::Creature, 78);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(321);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_level(42);

    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(map.get_creature(guid).unwrap().guid(), guid);
    assert_eq!(
        map.get_typed_creature(guid).unwrap().unit().data().level,
        42
    );
    map.get_typed_creature_mut(guid)
        .unwrap()
        .unit_mut()
        .set_level(43);
    assert_eq!(
        map.get_typed_creature(guid).unwrap().unit().data().level,
        43
    );
}
#[test]
fn add_map_object_record_to_map_like_cpp_preserves_typed_creature_spawn_index() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(396, 39601, false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.inserted);
    assert!(!outcome.already_in_world);
    assert!(outcome.inserted_into_cell);
    assert!(map.get_creature_by_spawn_id_like_cpp(396).is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .is_some()
    );

    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.grid_objects.creatures.contains(&guid));
    assert!(!cell.world_objects.creatures.contains(&guid));
}
#[test]
fn creature_search_formation_add_to_map_inserts_group_holder_and_coexists_with_vehicle_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(470, 47001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900470)));
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9470);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let search = outcome.creature_search_formation.unwrap();
    assert_eq!(search.spawn_id, 470);
    assert_eq!(search.leader_spawn_id, Some(900470));
    assert!(search.add_to_group_requested);
    assert!(map.creature_group_holder_contains_like_cpp(900470, guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900470), 1);
    assert!(outcome.creature_vehicle_reset.is_some());
    assert!(outcome.creature_vehicle_install.is_some());
}
#[test]
fn creature_search_formation_add_to_map_removes_stale_same_spawn_member_like_cpp() {
    let mut map = test_map();
    let mut old_creature = test_creature_for_spawn(471, 47101, true);
    old_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    old_creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900471)));
    let old_guid = old_creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(old_creature).unwrap())
        .unwrap();
    assert!(map.creature_group_holder_contains_like_cpp(900471, old_guid));

    let mut new_creature = test_creature_for_spawn(471, 47102, true);
    new_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    new_creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900471)));
    let new_guid = new_creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(new_creature).unwrap())
        .unwrap();

    assert!(
        outcome
            .creature_search_formation
            .as_ref()
            .is_some_and(|search| search.add_to_group_requested)
    );
    assert!(!map.creature_group_holder_contains_like_cpp(900471, old_guid));
    assert!(map.creature_group_holder_contains_like_cpp(900471, new_guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900471), 1);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(471), 2);
}
#[test]
fn creature_search_formation_add_to_map_already_in_world_is_not_consumed_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(472, 47201, true);
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900472)));
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(outcome.already_in_world);
    assert!(outcome.creature_search_formation.is_none());
    assert!(!map.creature_group_holder_contains_like_cpp(900472, guid));
}
#[test]
fn creature_search_formation_add_to_map_non_creature_path_is_unchanged_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(473, 47301);
    gameobject.world_mut().object_mut().remove_from_world();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.creature_search_formation.is_none());
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900473), 0);
}
#[test]
fn creature_search_formation_remove_from_map_removes_last_member_and_holder_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(474, 47401, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900474)));
    let guid = creature.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.creature_group_holder_contains_like_cpp(900474, guid));

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();
    let formation = removed.creature_remove_formation.unwrap();

    assert_eq!(formation.guid, guid);
    assert_eq!(formation.spawn_id, 474);
    assert_eq!(formation.leader_spawn_id, Some(900474));
    assert!(formation.had_group);
    assert!(formation.removed_member);
    assert!(formation.removed_group);
    assert_eq!(formation.remaining_members, 0);
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900474), 0);
}
#[test]
fn creature_search_formation_remove_from_map_keeps_group_with_other_member_like_cpp() {
    let mut map = test_map();
    let mut first = test_creature_for_spawn(475, 47501, true);
    first
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    first.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900475)));
    let first_guid = first.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(first).unwrap())
        .unwrap();

    let mut second = test_creature_for_spawn(476, 47601, true);
    second
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    second.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900475)));
    let second_guid = second.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(second).unwrap())
        .unwrap();
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900475), 2);

    let removed = map.remove_from_map_like_cpp(first_guid, true).unwrap();
    let formation = removed.creature_remove_formation.unwrap();

    assert!(formation.had_group);
    assert!(formation.removed_member);
    assert!(!formation.removed_group);
    assert_eq!(formation.remaining_members, 1);
    assert!(!map.creature_group_holder_contains_like_cpp(900475, first_guid));
    assert!(map.creature_group_holder_contains_like_cpp(900475, second_guid));
}
#[test]
fn creature_search_formation_remove_from_map_existing_holder_non_member_keeps_holder_like_cpp() {
    let mut map = test_map();
    let mut member = test_creature_for_spawn(483, 48301, true);
    member
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    member.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900483)));
    let member_guid = member.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(member).unwrap())
        .unwrap();
    let member_count_before = map.creature_group_holder_member_count_like_cpp(900483);
    assert_eq!(member_count_before, 1);
    assert!(map.creature_group_holder_contains_like_cpp(900483, member_guid));

    let mut non_member = test_creature_for_spawn(484, 48401, true);
    non_member.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900483)));
    let non_member_guid = non_member.guid();
    let add_outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(non_member).unwrap())
        .unwrap();
    assert!(add_outcome.already_in_world);
    assert!(!map.creature_group_holder_contains_like_cpp(900483, non_member_guid));

    let removed = map.remove_from_map_like_cpp(non_member_guid, true).unwrap();
    let formation = removed.creature_remove_formation.unwrap();

    assert!(formation.had_group);
    assert!(!formation.removed_member);
    assert!(!formation.removed_group);
    assert_eq!(formation.remaining_members, member_count_before);
    assert!(map.creature_group_holder_contains_like_cpp(900483, member_guid));
    assert!(!map.creature_group_holder_contains_like_cpp(900483, non_member_guid));
}
#[test]
fn creature_search_formation_remove_from_map_no_formation_or_not_in_world_noops_like_cpp() {
    let mut map = test_map();
    let mut holder_creature = test_creature_for_spawn(477, 47701, true);
    holder_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    holder_creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900477)));
    let holder_guid = holder_creature.guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_creature(holder_creature).unwrap(),
    )
    .unwrap();

    let mut no_formation = test_creature_for_spawn(478, 47801, true);
    no_formation
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let no_formation_guid = no_formation.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(no_formation).unwrap())
        .unwrap();
    let removed = map
        .remove_from_map_like_cpp(no_formation_guid, true)
        .unwrap();
    assert!(removed.creature_remove_formation.is_none());
    assert!(map.creature_group_holder_contains_like_cpp(900477, holder_guid));

    let mut missing_holder = test_creature_for_spawn(479, 47901, true);
    missing_holder.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900479)));
    let missing_holder_guid = missing_holder.guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_creature(missing_holder).unwrap(),
    )
    .unwrap();
    let removed = map
        .remove_from_map_like_cpp(missing_holder_guid, true)
        .unwrap();
    let formation = removed.creature_remove_formation.unwrap();
    assert_eq!(formation.leader_spawn_id, Some(900479));
    assert!(!formation.had_group);
    assert!(!formation.removed_member);
    assert!(!formation.removed_group);
    assert_eq!(formation.remaining_members, 0);
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900479), 0);
    assert!(map.creature_group_holder_contains_like_cpp(900477, holder_guid));

    let mut not_in_world = test_creature_for_spawn(482, 48201, true);
    not_in_world.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900477)));
    let not_in_world_guid = not_in_world.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(not_in_world).unwrap())
        .unwrap();
    map.entity_world
        .get_mut(&not_in_world_guid)
        .and_then(MapObjectRecord::creature_mut)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let removed = map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();
    assert!(removed.creature_remove_formation.is_none());
    assert!(map.creature_group_holder_contains_like_cpp(900477, holder_guid));
}
#[test]
fn creature_search_formation_remove_from_map_non_creature_path_is_unchanged_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(480, 48001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900480)));
    let creature_guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let mut gameobject = test_gameobject_for_spawn(481, 48101);
    gameobject.world_mut().object_mut().remove_from_world();
    let gameobject_guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let removed = map.remove_from_map_like_cpp(gameobject_guid, true).unwrap();

    assert!(removed.creature_remove_formation.is_none());
    assert!(map.creature_group_holder_contains_like_cpp(900480, creature_guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900480), 1);
}
#[test]
fn creature_add_to_world_unit_seam_only_for_exact_typed_creature_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(475, 47501, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let caster = ObjectGuid::new(0, 47599);
    let enter_world_aura = AppliedAuraRef::new(47_510, caster, 1, 0x1);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .register_applied_aura(
            enter_world_aura,
            None,
            SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
            0,
        );
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(
        outcome.creature_store_inserted_before_add_to_world,
        Some(true)
    );
    assert_eq!(
        outcome.creature_spawn_indexed_before_add_to_world,
        Some(true)
    );
    let unit_add = outcome.creature_unit_add_to_world.unwrap();
    assert_eq!(unit_add.guid, guid);
    assert!(unit_add.world_object_added);
    assert!(unit_add.is_in_world_after);
    assert_eq!(unit_add.removed_enter_world_auras, vec![enter_world_aura]);
    assert!(
        unit_add
            .motion_master_add_to_world
            .had_initialization_pending
    );
    assert!(
        unit_add
            .motion_master_add_to_world
            .direct_initialize_represented
    );
    assert!(outcome.creature_zone_script_create.is_some());
    assert!(
        map.map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .is_some_and(|creature| !creature
                .unit()
                .subsystems()
                .auras
                .has_applied(enter_world_aura))
    );

    let generic_creature = world_object_with_counter(HighGuid::Creature, 47502, 571, 7, false);
    let generic = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new(AccessorObjectKind::Creature, generic_creature).unwrap(),
        )
        .unwrap();
    assert!(
        generic
            .creature_store_inserted_before_add_to_world
            .is_none()
    );
    assert!(generic.creature_spawn_indexed_before_add_to_world.is_none());
    assert!(generic.creature_unit_add_to_world.is_none());
    assert!(generic.creature_search_formation.is_none());
    assert!(generic.creature_aim_initialize.is_none());
    assert!(generic.creature_zone_script_create.is_none());

    let gameobject = test_gameobject_for_spawn(47503, 47504);
    let non_creature = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    assert!(
        non_creature
            .creature_store_inserted_before_add_to_world
            .is_none()
    );
    assert!(
        non_creature
            .creature_spawn_indexed_before_add_to_world
            .is_none()
    );
    assert!(non_creature.creature_unit_add_to_world.is_none());
    assert!(non_creature.creature_aim_initialize.is_none());
}
#[test]
fn creature_aim_initialize_add_to_map_emits_for_normal_creature_without_vehicle_reset_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(476, 47601, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let aim = outcome.creature_aim_initialize.unwrap();
    assert_eq!(aim.guid, guid);
    assert_eq!(aim.spawn_id, 476);
    assert!(aim.aim_create_represented);
    assert!(aim.motion_initialize_represented);
    assert!(!aim.formation_present);
    assert!(!aim.formation_leader);
    assert!(!aim.formation_move_idle_represented);
    assert!(!aim.motion_initialize_requires_formed_state);
    assert!(aim.motion_master_initialize_represented);
    assert!(aim.ai_selected_represented);
    assert!(aim.ai_initialize_represented);
    assert!(!aim.vehicle_reset_expected);
    assert!(aim.succeeded);
    assert!(outcome.creature_vehicle_reset.is_none());
}
#[test]
fn creature_zone_script_add_to_map_create_evidence_only_on_normal_creature_path_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(490, 49001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let zone_script = outcome.creature_zone_script_create.unwrap();
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback);
    assert!(!zone_script.script_dispatch_represented);
    assert!(!outcome.already_in_world);

    let already_in_world = test_creature_for_spawn(491, 49101, true);
    let already = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(already_in_world).unwrap(),
        )
        .unwrap();

    assert!(already.already_in_world);
    assert!(already.creature_zone_script_create.is_none());

    let generic_creature = world_object_with_counter(HighGuid::Creature, 49002, 571, 7, false);
    let generic = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new(AccessorObjectKind::Creature, generic_creature).unwrap(),
        )
        .unwrap();

    assert!(!generic.already_in_world);
    assert!(generic.creature_zone_script_create.is_none());
}
#[test]
fn creature_zone_script_add_to_map_create_follows_vehicle_install_tail_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(492, 49201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9492, true);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let install = outcome.creature_vehicle_install.unwrap();
    assert_eq!(install.kit_id, Some(9492));
    let zone_script = outcome.creature_zone_script_create.unwrap();
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback);
    assert!(!zone_script.script_dispatch_represented);
}
#[test]
fn creature_zone_script_remove_from_map_remove_evidence_precedes_formation_then_unit_vehicle_like_cpp()
 {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(493, 49301, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_formation_info_like_cpp(Some(creature_formation_info_like_cpp(900493)));
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .set_vehicle_kit(9493, true);
    let guid = creature.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    assert!(map.creature_group_holder_contains_like_cpp(900493, guid));

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();

    let zone_script = removed.creature_zone_script_remove.unwrap();
    assert_eq!(zone_script.guid, guid);
    assert!(zone_script.represented_callback);
    assert!(!zone_script.script_dispatch_represented);
    let formation = removed.creature_remove_formation.unwrap();
    assert_eq!(formation.guid, guid);
    assert!(formation.had_group);
    assert!(formation.removed_member);
    assert!(formation.removed_group);
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900493), 0);
    let unit_remove = removed.creature_unit_remove_from_world.unwrap();
    assert_eq!(unit_remove.guid, guid);
    assert!(unit_remove.was_in_world);
    assert!(unit_remove.during_remove_entered);
    assert!(!unit_remove.ai_on_despawn_represented);
    assert!(!unit_remove.leave_world_cleanup_represented);
    assert!(unit_remove.world_object_removed);
    assert!(unit_remove.during_remove_cleared);
    let unit_vehicle_remove = unit_remove.vehicle_remove.unwrap();
    assert_eq!(unit_vehicle_remove.kit_id, Some(9493));
    assert_eq!(removed.creature_vehicle_remove, Some(unit_vehicle_remove));
}
#[test]
fn creature_zone_script_remove_from_map_missing_not_in_world_and_non_creature_noop_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Creature, 49401);
    assert!(matches!(
        map.remove_from_map_like_cpp(missing_guid, true),
        Err(RemoveFromMapError::ObjectNotFound { guid }) if guid == missing_guid
    ));

    let mut not_in_world = test_creature_for_spawn(494, 49402, true);
    not_in_world
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let not_in_world_guid = not_in_world.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(not_in_world).unwrap())
        .unwrap();
    map.entity_world
        .get_mut(&not_in_world_guid)
        .and_then(MapObjectRecord::creature_mut)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let removed = map
        .remove_from_map_like_cpp(not_in_world_guid, true)
        .unwrap();
    assert!(!removed.was_in_world);
    assert!(removed.creature_zone_script_remove.is_none());
    assert!(removed.creature_remove_formation.is_none());
    assert!(removed.creature_unit_remove_from_world.is_none());
    assert!(removed.creature_vehicle_remove.is_none());

    let mut gameobject = test_gameobject_for_spawn(495, 49501);
    gameobject.world_mut().object_mut().remove_from_world();
    let gameobject_guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    let removed = map.remove_from_map_like_cpp(gameobject_guid, true).unwrap();
    assert!(removed.creature_zone_script_remove.is_none());
}
#[test]
fn creature_vehicle_add_to_map_resets_then_installs_vehicle_kit_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(400, 40001, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9003);
    let guid = creature.guid();

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let reset = outcome.creature_vehicle_reset.unwrap();
    assert_eq!(reset.kit_id, 9003);
    let aim = outcome.creature_aim_initialize.unwrap();
    assert_eq!(aim.guid, guid);
    assert_eq!(aim.spawn_id, 400);
    assert!(!aim.motion_initialize_requires_formed_state);
    assert!(aim.vehicle_reset_expected);
    assert!(aim.aim_create_represented);
    assert!(aim.ai_initialize_represented);
    assert!(reset.aim_create_represented);
    assert!(reset.ai_initialize_represented);
    assert!(!reset.reset_evading);
    assert!(reset.reset_plan.call_on_reset_script);
    assert!(
        reset
            .reset_plan
            .immunity_plan
            .immunities
            .contains(&VehicleSpellImmunity {
                kind: VehicleSpellImmunityKind::Effect,
                spell_or_mechanic: 98,
                apply: true,
            })
    );
    let accessory_plan = reset.reset_plan.accessory_install_plan.unwrap();
    assert_eq!(accessory_plan.accessories.len(), 1);
    assert_eq!(accessory_plan.accessories[0].accessory_entry, 7001);
    assert!(outcome.creature_vehicle_install.is_some());
    let stored = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    let kit = stored.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id(), 9003);
    assert!(kit.installed());
}
#[test]
fn creature_vehicle_add_to_map_mechanical_world_boss_skips_mechanical_immunities_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(401, 40101, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(true, true),
    ));
    create_loaded_creature_vehicle_kit_like_cpp(&mut creature, 9004);

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let immunities = &outcome
        .creature_vehicle_reset
        .unwrap()
        .reset_plan
        .immunity_plan
        .immunities;
    assert!(immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::Effect,
        spell_or_mechanic: 98,
        apply: true,
    }));
    assert!(!immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::Effect,
        spell_or_mechanic: 6,
        apply: true,
    }));
}
#[test]
fn creature_vehicle_add_to_map_without_kit_has_no_reset_evidence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(402, 40201, true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        creature_add_to_world_vehicle_reset_context(false, false),
    ));

    let outcome = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(!outcome.already_in_world);
    assert!(outcome.creature_vehicle_reset.is_none());
}
