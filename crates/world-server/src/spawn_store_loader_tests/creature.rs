//! Creature scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn waypoint_path_store_initializes_world_creature_default_waypoint_like_cpp() {
    let (store, _report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [WaypointPathRowLikeCpp {
            path_id: 30,
            move_type: 1,
            flags: 0,
        }],
        [WaypointPathNodeRowLikeCpp {
            path_id: 30,
            node_id: 7,
            x: 11.0,
            y: 12.0,
            z: 13.0,
            orientation: Some(1.5),
            delay: 250,
        }],
    );
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54_340);
    let mut creature = wow_world::map_manager::WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.creature.load_path_like_cpp(30);

    let action =
        initialize_world_creature_default_waypoint_from_store_like_cpp(&mut creature, &store);

    assert_eq!(action, wow_movement::WaypointMovementAction::StopMoving);
    assert!(creature.creature.unit().subsystems().motion.stopped);
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        wow_movement::WaypointMovementAction::Launch(launch)
            if launch.path_id == 30
                && launch.node_id == 7
                && launch.destination == Position::new(11.0, 12.0, 13.0, 0.0)
    ));
}
#[test]
fn creature_spawn_equipment_random_is_normalized_before_runtime_row_like_cpp() {
    let mut row = creature_row(1, 0, "0");
    row.entry = 123;
    row.equipment_id = -1;
    let equipment_store = wow_data::CreatureEquipmentStoreLikeCpp::from_entries([(
        123,
        1,
        wow_data::CreatureEquipmentInfoLikeCpp::default(),
    )]);

    normalize_creature_spawn_equipment_id_like_cpp(&mut row, &equipment_store);
    let runtime = creature_row_to_runtime_row_like_cpp(&row);

    assert_eq!(row.equipment_id, 1);
    assert_eq!(runtime.equipment_id, 1);
}
#[test]
fn creature_spawn_missing_equipment_is_normalized_to_zero_like_cpp() {
    let mut row = creature_row(1, 0, "0");
    row.entry = 123;
    row.equipment_id = 7;
    let equipment_store = wow_data::CreatureEquipmentStoreLikeCpp::from_entries([(
        123,
        1,
        wow_data::CreatureEquipmentInfoLikeCpp::default(),
    )]);

    normalize_creature_spawn_equipment_id_like_cpp(&mut row, &equipment_store);

    assert_eq!(row.equipment_id, 0);
}
#[test]
fn game_event_spawn_guids_preserve_creature_and_gameobject_order_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut creature_report = GameEventObjectGuidLoadReportLikeCpp::default();
    let mut gameobject_report = GameEventObjectGuidLoadReportLikeCpp::default();

    for row in [
        GameEventObjectGuidRowLikeCpp {
            guid: 100,
            event_id: 1,
        },
        GameEventObjectGuidRowLikeCpp {
            guid: 102,
            event_id: 1,
        },
    ] {
        apply_game_event_object_guid_row_like_cpp(
            row,
            SpawnObjectType::Creature,
            &store,
            &mut guids,
            &mut creature_report,
        );
    }
    for row in [
        GameEventObjectGuidRowLikeCpp {
            guid: 200,
            event_id: -1,
        },
        GameEventObjectGuidRowLikeCpp {
            guid: 202,
            event_id: -1,
        },
    ] {
        apply_game_event_object_guid_row_like_cpp(
            row,
            SpawnObjectType::GameObject,
            &store,
            &mut guids,
            &mut gameobject_report,
        );
    }

    assert_eq!(
        guids.creature_guids_like_cpp(1),
        Some([100, 102].as_slice())
    );
    assert_eq!(
        guids.gameobject_guids_like_cpp(-1),
        Some([200, 202].as_slice())
    );
    assert_eq!(creature_report.rows, 2);
    assert_eq!(creature_report.loaded, 2);
    assert_eq!(gameobject_report.rows, 2);
    assert_eq!(gameobject_report.loaded, 2);
}
#[test]
fn game_event_quest_creature_preserves_order_duplicates_and_get_uint8_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    for row in [
        game_event_quest_row(2, 100, 7000),
        game_event_quest_row(2, 100, 7000),
        game_event_quest_row(2, 101, 7001),
        game_event_quest_row_from_raw_event_entry_get_uint8_like_cpp(258, 102, 7002),
    ] {
        apply_game_event_creature_quest_relation_row_like_cpp(row, &mut quests, &mut report);
    }

    let records = quests.creature_records_like_cpp(2).unwrap();
    assert_eq!(
        records,
        &[
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 100,
                quest_id: 7000,
            },
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 100,
                quest_id: 7000,
            },
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 101,
                quest_id: 7001,
            },
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 102,
                quest_id: 7002,
            },
        ]
    );
    assert_eq!(report.loaded, 4);
    assert_eq!(report.skipped_out_of_range, 0);
}
#[test]
fn game_event_npc_vendor_missing_creature_metadata_skips_no_dummy_like_cpp() {
    let store = game_event_npc_vendor_store(&[]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(1, 404, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    assert_eq!(vendors.records_like_cpp(1).unwrap(), &[]);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_missing_creature_spawn_metadata, 1);
    assert_eq!(report.validation_deferred, 0);
}
#[test]
fn linked_respawn_loader_validation_valid_creature_to_gameobject_inserts_like_cpp() {
    let maps = instanceable_map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut kind_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let slave = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    let master = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(200, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    store.add_object_spawn(&slave, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&master, is_personal_phase_like_cpp_represented);
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let mut report = LinkedRespawnLoadReportLikeCpp::default();

    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: LinkedRespawnTypeLikeCpp::CreatureToGameObject as u8,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );

    assert_eq!(report.inserted, 1);
    assert_eq!(linked_store.len(), 1);
    let slave_guid = spawn_data_guid_like_cpp(&slave);
    let master_guid = spawn_data_guid_like_cpp(&master);
    assert_eq!(
        linked_store.get_linked_respawn_guid_like_cpp(slave_guid),
        master_guid
    );
}
#[test]
fn creature_row_indexes_only_non_event_rows_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();

    let indexed = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .expect("non-event creature spawn should convert");
    store.add_object_spawn(&indexed, is_personal_phase_like_cpp_represented);

    let event_managed = creature_row_to_spawn_data_like_cpp(
        &creature_row(101, 7, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .expect("event-managed creature spawn metadata should convert");
    store.insert_spawn_metadata_like_cpp(&event_managed);

    assert!(
        store
            .cell_object_guids(1, 0, indexed.cell_id())
            .is_some_and(|cell| cell.creatures.contains(&100))
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 101)
            .map(|spawn| spawn.spawn_id),
        Some(101)
    );
    assert!(
        store
            .cell_object_guids(1, 0, event_managed.cell_id())
            .is_none_or(|cell| !cell.creatures.contains(&101))
    );
}
#[test]
fn creature_formation_loader_converts_member_degrees_to_radians_like_cpp() {
    let store = formation_test_store(&[10, 11]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [
            formation_row(10, 10, 99.0, 180.0),
            formation_row(10, 11, 7.5, 90.0),
        ],
        &store,
        &mut report,
    );

    let member = formations.get(&11).expect("member formation should load");
    assert_eq!(member.leader_spawn_id, 10);
    assert_eq!(member.follow_dist, 7.5);
    assert!((member.follow_angle_radians - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    assert_eq!(member.group_ai, 17);
    assert_eq!(member.leader_waypoint_ids, [101, 102]);
    assert_eq!(report.loaded, 2);
}
#[test]
fn creature_formation_loader_forces_leader_self_dist_angle_zero_like_cpp() {
    let store = formation_test_store(&[20]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [formation_row(20, 20, 33.0, 270.0)],
        &store,
        &mut report,
    );

    let leader = formations.get(&20).expect("leader self row should load");
    assert_eq!(leader.follow_dist, 0.0);
    assert_eq!(leader.follow_angle_radians, 0.0);
    assert_eq!(report.loaded, 1);
}
#[test]
fn creature_formation_loader_skips_missing_leader_and_member_like_cpp() {
    let store = formation_test_store(&[30, 31]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [
            formation_row(99, 31, 1.0, 1.0),
            formation_row(30, 98, 1.0, 1.0),
            formation_row(30, 30, 0.0, 0.0),
        ],
        &store,
        &mut report,
    );

    assert!(formations.contains_key(&30));
    assert_eq!(formations.len(), 1);
    assert_eq!(report.rows, 3);
    assert_eq!(report.skipped_missing_leader, 1);
    assert_eq!(report.skipped_missing_member, 1);
}
#[test]
fn creature_formation_loader_prunes_group_without_leader_self_row_like_cpp() {
    let store = formation_test_store(&[40, 41]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [formation_row(40, 41, 4.0, 45.0)],
        &store,
        &mut report,
    );

    assert!(formations.is_empty());
    assert_eq!(report.removed_missing_leader_self, 1);
    assert_eq!(report.loaded, 0);
}
#[test]
fn creature_formation_loader_duplicate_member_keeps_first_like_cpp_emplace() {
    let store = formation_test_store(&[50, 51]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [
            formation_row(50, 50, 0.0, 0.0),
            formation_row(50, 51, 3.0, 30.0),
            formation_row(50, 51, 9.0, 90.0),
        ],
        &store,
        &mut report,
    );

    let member = formations.get(&51).expect("first member row should remain");
    assert_eq!(member.follow_dist, 3.0);
    assert!(
        (member.follow_angle_radians - (30.0_f32 * std::f32::consts::PI / 180.0)).abs() < 0.0001
    );
    assert_eq!(report.duplicate_member_ignored, 1);
    assert_eq!(report.loaded, 2);
}
#[test]
fn templates_and_spawn_group_apply_cover_creature_go_at_and_event_gap() {
    let (template_store, _) = wow_data::SpawnGroupTemplateStore::from_rows_like_cpp([
        wow_data::SpawnGroupTemplateRow {
            group_id: 10,
            name: "custom".to_string(),
            flags: 0,
        },
        wow_data::SpawnGroupTemplateRow {
            group_id: 11,
            name: "manual".to_string(),
            flags: wow_data::spawn_group::SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP,
        },
    ]);
    let mut templates = spawn_group_templates_for_spawn_store(&template_store);
    assert_eq!(templates.get(&0).unwrap().map_id, 0);
    assert_eq!(templates.get(&1).unwrap().map_id, 0);
    assert_eq!(templates.get(&10).unwrap().map_id, SPAWNGROUP_MAP_UNSET);

    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let area_trigger_templates = valid_area_trigger_template_store();
    let mut area_trigger_runtime_rows = BTreeMap::new();

    let creature = creature_row_to_spawn_data_like_cpp(
        &creature_row(300, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let go = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(301, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let at = area_trigger_row_to_spawn_data_like_cpp(
        &area_trigger_row(302, "0"),
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |_| true,
        &mut |_| wow_data::ScriptIdLikeCpp(0),
        &mut area_trigger_runtime_rows,
        &mut report,
    )
    .unwrap();
    let event_managed = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(303, 5, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();

    store.add_object_spawn(&creature, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&go, is_personal_phase_like_cpp_represented);
    store.add_area_trigger_spawn(&at);
    store.insert_spawn_metadata_like_cpp(&event_managed);

    let apply = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 300,
            },
            SpawnGroupMemberRow {
                group_id: 11,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 301,
            },
            SpawnGroupMemberRow {
                group_id: 1,
                spawn_type: SpawnObjectType::AreaTrigger as u8,
                spawn_id: 302,
            },
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: event_managed.spawn_id,
            },
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 999,
            },
        ],
    );

    assert_eq!(apply.assigned, 3);
    assert_eq!(apply.missing_spawn, 1);
    assert_eq!(apply.duplicate_spawn_group, 1);
    assert_eq!(templates.get(&0).unwrap().map_id, 0);
    assert_eq!(templates.get(&1).unwrap().map_id, 0);
    assert_eq!(templates.get(&10).unwrap().map_id, 1);
    assert_eq!(templates.get(&11).unwrap().map_id, 1);
    assert!(templates.contains_key(&0));
    assert!(templates.contains_key(&1));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(store.clone(), templates.clone());
    assert_eq!(metadata.spawn_group_templates().get(&10).unwrap().map_id, 1);
    assert!(metadata.spawn_group_templates().contains_key(&0));
    assert!(metadata.spawn_group_templates().contains_key(&1));
    assert_eq!(
        metadata
            .spawn_store()
            .spawn_group_ids_by_map(1)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 300)
            .unwrap()
            .spawn_group_id(),
        10
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::GameObject, 301)
            .unwrap()
            .spawn_group_id(),
        11
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::AreaTrigger, 302)
            .unwrap()
            .spawn_group_id(),
        1
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::GameObject, 303)
            .unwrap()
            .spawn_group_id(),
        10
    );
    assert!(
        store
            .cell_object_guids(1, 0, event_managed.cell_id())
            .is_none_or(|cell| !cell.gameobjects.contains(&303))
    );
}
