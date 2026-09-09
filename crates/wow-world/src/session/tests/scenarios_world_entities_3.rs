//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn talked_to_creature_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(9_902);
    let quest_id = 12_505;
    let creature_entry = 9_903;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 3, // C++ QUEST_OBJECTIVE_TALKTO.
        order: 0,
        storage_index: 0,
        object_id: creature_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session
        .talked_to_creature_like_cpp(creature_entry, creature_guid)
        .await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        session.represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: false,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[test]
fn session_wires_gameobject_template_lifecycle_store_for_spell_summons_like_cpp() {
    let (mut session, _, _) = make_session();
    let store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry: 7001,
                go_type: 6,
                display_id: 44,
                name: "spell summoned gameobject".to_string(),
                size: 1.0,
                data: [0; wow_entities::MAX_GAMEOBJECT_DATA],
                content_tuning_id: 80,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: Some(wow_data::GameObjectTemplateAddonLifecycleRecordLikeCpp {
                    entry: 7001,
                    faction: 35,
                    flags: 0x20,
                    world_effect_id: 9,
                    anim_kit_id: 3,
                }),
            },
        ]),
    );

    assert!(session.gameobject_template_lifecycle_store().is_none());
    session.set_gameobject_template_lifecycle_store(Arc::clone(&store));

    let template = session
        .gameobject_template_lifecycle_store()
        .and_then(|store| store.get(7001))
        .expect("template store should be available to spell summon callers");
    let lifecycle_record = wow_data::gameobject_template_lifecycle_record_like_cpp(template);
    assert_eq!(lifecycle_record.entry, 7001);
    assert_eq!(lifecycle_record.go_type, 6);
    assert_eq!(lifecycle_record.faction, 35);
    assert_eq!(lifecycle_record.flags, 0x20);
    assert_eq!(lifecycle_record.level, 80);
    assert_eq!(lifecycle_record.world_effect_id, 9);
    assert_eq!(lifecycle_record.anim_kit_id, 3);
}
#[tokio::test]
async fn summon_object_wild_live_spell_without_focus_creates_visible_gameobject_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 702_i32;
    let template_entry = 9003_u32;
    let player_guid = ObjectGuid::create_player(1, 7004);
    let player_position = Position::new(100.0, 200.0, 30.0, 0.0);
    let canonical = shared_canonical_map_manager();
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(
            spell_id,
            0,
            vec![summon_object_wild_effect_like_cpp(
                i32::try_from(template_entry).unwrap(),
            )],
        ),
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 702,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("live non-focus wild GameObject summon should execute");

    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .find(ObjectGuid::is_game_object)
        .expect("force visibility should make the newly summoned GameObject visible");
    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    assert_eq!(managed.map().map_object_count(), 2);
    let gameobject = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("visible summoned GO should be map-owned");
    assert_eq!(gameobject.world().object().entry(), template_entry);
    assert_eq!(
        gameobject.world().position(),
        Position::new(
            player_position.x + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP,
            player_position.y,
            player_position.z,
            player_position.orientation
        )
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "live summon should trigger represented visibility create/update delivery"
    );
}
#[test]
fn search_spell_focus_requires_type_spawned_and_radius_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 7007);
    let player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let matching_focus = test_gameobject_guid(9010, 7010);
    let wrong_type_focus = test_gameobject_guid(9011, 7011);
    let far_focus = test_gameobject_guid(9012, 7012);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FocusCaster".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        wrong_type_focus,
        9_011,
        182,
        50,
        Position::new(11.0, 20.0, 30.0, 0.25),
        571,
        0,
    );
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        far_focus,
        9_012,
        181,
        2,
        Position::new(40.0, 20.0, 30.0, 0.5),
        571,
        0,
    );
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        matching_focus,
        9_010,
        181,
        10,
        Position::new(12.0, 20.0, 30.0, 1.25),
        571,
        0,
    );

    let focus = session
        .search_spell_focus_like_cpp(181)
        .expect("matching spell focus should be found");
    assert_eq!(focus.guid, matching_focus);
    assert_eq!(focus.source.focus_type, 181);
    assert_eq!(focus.source.radius, 10);
    assert_eq!(focus.position.orientation, 1.25);
    assert!(session.search_spell_focus_like_cpp(999).is_none());
}
#[tokio::test]
async fn creature_cast_target_dest_home_keeps_creature_destination_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 0]));
    let spell_id = 86_902_i32;
    let player_guid = ObjectGuid::create_player(1, 7043);
    let creature_guid = test_creature_guid(7043);
    let vehicle_base_guid = test_vehicle_guid(7044);
    let transport_guid = ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 7043);
    let instance_id = 42_u32;
    let player_position = Position::new(10.0, 20.0, 30.0, 0.5);
    let creature_position = Position::new(70.0, 80.0, 90.0, 1.5);
    let vehicle_base_position = Position::new(60.0, 65.0, 75.0, 0.75);
    let vehicle_offset =
        wow_entities::calculate_passenger_offset(creature_position, vehicle_base_position);
    let transport_position = Position::new(50.0, 60.0, 70.0, 0.5);
    let stale_instance_copy_position = Position::new(270.0, 280.0, 290.0, 2.0);
    let base_map_copy_position = Position::new(170.0, 180.0, 190.0, 2.5);
    let home_position = Position::new(40.0, 50.0, 60.0, 1.25);
    let home_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_HOME,
        ..Default::default()
    };

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CreatureHomeCaster".to_string(),
        player_position,
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 1,
        area_id: 1519,
        position: home_position,
    });
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 0, instance_id);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9001,
        creature_position,
        0,
        0,
        instance_id,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        vehicle_base_guid,
        9002,
        vehicle_base_position,
        0,
        0,
        instance_id,
    );
    {
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, instance_id)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(creature_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .vehicle
            .enter_vehicle(vehicle_base_guid, Some(0));
        let mut transport = wow_entities::Transport::new();
        transport.world_mut().object_mut().create(transport_guid);
        transport.world_mut().set_map(0, instance_id).unwrap();
        transport.world_mut().relocate(transport_position);
        transport.world_mut().object_mut().add_to_world();
        assert!(transport.add_passenger(creature_guid));
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, instance_id)
            .unwrap()
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_transport(transport).unwrap(),
            )
            .unwrap();
    }
    session.set_canonical_map_manager(canonical);
    let explicit_transport_offset = Position::new(5.0, -3.0, 2.0, 0.25);
    assert_eq!(
        session.represented_transport_destination_world_position_like_cpp(
            transport_guid,
            explicit_transport_offset,
        ),
        Some(wow_entities::calculate_passenger_position(
            explicit_transport_offset,
            transport_position,
        )),
        "an explicit transported destination resolves its own offset, not the caster position"
    );
    let manager = shared_map_manager();
    {
        let mut manager_guard = manager.write().unwrap();
        for (copy_instance_id, position) in [
            (0_u32, base_map_copy_position),
            (instance_id, stale_instance_copy_position),
        ] {
            let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
            assert!(manager_guard.add_creature(
                0,
                copy_instance_id,
                grid_x,
                grid_y,
                crate::map_manager::WorldCreature::new(
                    creature_guid,
                    9001,
                    position,
                    100,
                    2,
                    3,
                    5,
                    20.0,
                    100,
                    14,
                    0,
                    0,
                ),
            ));
        }
    }
    session.set_map_manager(manager);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        teleport_units_spell_info_like_cpp(spell_id, vec![home_effect]),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
            SpellCastMetadata {
                caster_guid_override: Some(creature_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("creature TARGET_DEST_HOME cast should execute");

    let bytes = send_rx.try_recv().expect("creature-cast SpellGo");
    let packet_target = decode_spell_go_target_data_like_cpp(&bytes, spell_id);
    let packet_destination = packet_target
        .dst_location
        .expect("creature's initial destination")
        .clone();
    assert_eq!(
        packet_destination.transport, vehicle_base_guid,
        "C++ Unit::GetTransGUID prioritizes the vehicle base over an ordinary transport"
    );
    assert_eq!(
        packet_destination.position,
        Position::new(vehicle_offset.x, vehicle_offset.y, vehicle_offset.z, 0.0),
        "C++ SpellCastTargets::Write serializes the caster vehicle offset"
    );
    assert_ne!(
        packet_destination.position,
        Position::new(home_position.x, home_position.y, home_position.z, 0.0)
    );
    assert_ne!(
        packet_destination.position,
        Position::new(
            stale_instance_copy_position.x,
            stale_instance_copy_position.y,
            stale_instance_copy_position.z,
            0.0
        ),
        "the canonical caster validated by the interaction path wins over a stale legacy copy"
    );
    assert_ne!(
        packet_destination.position,
        Position::new(
            base_map_copy_position.x,
            base_map_copy_position.y,
            base_map_copy_position.z,
            0.0
        ),
        "the same runtime GUID in instance 0 must not shadow the effective caster"
    );
    assert_eq!(
        session.pending_teleport_save_destination_like_cpp(),
        Some((0_u16, creature_position)),
        "effect execution keeps the global SpellDestination position"
    );
}
#[tokio::test]
async fn creature_cast_db_destination_without_row_keeps_effective_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 86_903_i32;
    let template_entry = 9021_u32;
    let player_guid = ObjectGuid::create_player(1, 7021);
    let creature_guid = test_creature_guid(7022);
    let player_position = Position::new(240.0, 340.0, 48.0, 0.25);
    let creature_position = Position::new(246.0, 344.0, 49.0, 1.75);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_DB;
    let spell_info = gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9022,
        creature_position,
        0,
        571,
        0,
    );

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
            SpellCastMetadata {
                caster_guid_override: Some(creature_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("creature TARGET_DEST_DB cast should execute without a DB row");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find_map(|guid| managed.map().get_typed_game_object(guid))
        .expect("creature-cast summon should be visible");
    assert_eq!(
        summoned.world().position(),
        creature_position,
        "C++ TARGET_DEST_DB starts from SpellDestination(*m_caster), not the logged-in player"
    );
    drop(manager);

    let bytes = send_rx.try_recv().expect("creature-cast SpellGo");
    let packet_target = decode_spell_go_target_data_like_cpp(&bytes, spell_id);
    assert_eq!(
        packet_target
            .dst_location
            .expect("effective caster destination")
            .position,
        Position::new(
            creature_position.x,
            creature_position.y,
            creature_position.z,
            0.0,
        )
    );
}
#[test]
fn represented_mount_aura_keeps_creature_vehicle_with_mount_display_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 12345)));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 7,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_x_display_store(Arc::new(wow_data::MountXDisplayStore::from_entries([
        wow_data::MountXDisplayEntry {
            id: 1,
            creature_display_info_id: 1000,
            player_condition_id: 0,
            mount_id: 7,
        },
    ])));
    session.set_creature_template_mount_store(Arc::new(
        wow_data::CreatureTemplateMountStoreLikeCpp::from_entries([
            wow_data::CreatureTemplateMountEntryLikeCpp {
                entry: 1234,
                vehicle_id: 55,
                models: vec![wow_data::CreatureTemplateMountModelLikeCpp {
                    display_id: 4321,
                    display_scale: 1.0,
                    probability: 0.0,
                }],
            },
        ]),
    ));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 1234,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    assert_eq!(session.player_mount_display_id_like_cpp, 1000);
    assert_eq!(session.player_mount_vehicle_id_like_cpp, 55);
    assert!(session.player_mounted_like_cpp);
    assert!(
        session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::PLAYER_CONTROLLED | UnitFlags::MOUNT)
    );
    assert_eq!(session.mount_vehicle_create_requests_like_cpp, 1);
}
#[tokio::test]
async fn dynamic_object_values_snapshot_creature_shared_vision_sends_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_529);
    let source_guid = test_creature_guid(50_530);
    let dynamic_guid = test_dynamic_object_guid(601_529, 50_531);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    let visibility_range = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .map()
        .visibility_range();
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_creature_on_map(
        &canonical,
        source_guid,
        601_529,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        0,
        571,
        7,
    );
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        source_guid,
        viewer_guid,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        viewer_guid,
        601_529,
        updated_position,
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 42.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_guid)
    );
}
#[tokio::test]
async fn gameobject_visibility_on_destroy_summary_visible_sends_destroy_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_505);
    let gameobject_guid = test_gameobject_guid(605_050, 50_506);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_visibility_on_destroy_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_050,
        5_050_506,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .last_game_objects_update_summary()
            .generic_visibility_on_destroy_guids
            .as_slice(),
        &[gameobject_guid]
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );

    session.process_pending().await;

    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert_eq!(
        session.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
}
#[tokio::test]
async fn gameobject_visibility_on_destroy_includes_dead_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_565);
    let gameobject_guid = test_gameobject_guid(605_065, 50_566);

    session.player_alive_like_cpp = false;
    session.player_health_like_cpp = 0;
    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_visibility_on_destroy_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_065,
        5_050_566,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .set_health(0);
    }
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .last_game_objects_update_summary()
            .generic_visibility_on_destroy_guids
            .as_slice(),
        &[gameobject_guid]
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
#[tokio::test]
async fn gameobject_visibility_on_destroy_not_in_world_player_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_567);
    let gameobject_guid = test_gameobject_guid(605_067, 50_568);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_visibility_on_destroy_gameobject_like_cpp(
        &canonical,
        gameobject_guid,
        605_067,
        5_050_568,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    assert_eq!(canonical.lock().unwrap().update(60_000), Some(60_000));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .last_game_objects_update_summary()
            .generic_visibility_on_destroy_guids
            .as_slice(),
        &[gameobject_guid]
    );
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
    }
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    assert_eq!(
        session.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
}
