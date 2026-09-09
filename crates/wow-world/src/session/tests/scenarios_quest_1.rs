//! Session scenarios exercising the represented quest responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn game_event_quest_complete_notify_reports_missing_sender_like_cpp() {
    let (session, _, _) = make_session();

    let outcome = session.notify_game_event_quest_complete_like_cpp(42).await;

    assert_eq!(
        outcome,
        GameEventQuestCompleteClientOutcomeLikeCpp::SenderMissing { quest_id: 42 }
    );
}
#[tokio::test]
async fn game_event_quest_complete_notify_sends_command_and_receives_response_like_cpp() {
    let (mut session, _, _) = make_session();
    let (command_tx, command_rx) = flume::bounded(1);
    session.set_game_event_quest_complete_sender_like_cpp(command_tx);

    let responder = tokio::spawn(async move {
        let command = command_rx.recv_async().await.expect("command received");
        assert_eq!(command.quest_id, 1234);
        command
            .response_tx
            .send_async(GameEventQuestCompleteResponseLikeCpp {
                quest_id: 1234,
                condition_save_updates_queued: 1,
                save_world_event_state_requested: true,
                world_event_state_save_requested: 1,
                force_game_event_update_requested: true,
                force_game_event_update_requests: 1,
                ..GameEventQuestCompleteResponseLikeCpp::default()
            })
            .await
            .expect("response sent");
    });

    let outcome = session
        .notify_game_event_quest_complete_like_cpp(1234)
        .await;
    responder.await.expect("responder task completed");

    match outcome {
        GameEventQuestCompleteClientOutcomeLikeCpp::Ok(response) => {
            assert_eq!(response.quest_id, 1234);
            assert_eq!(response.condition_save_updates_queued, 1);
            assert!(response.save_world_event_state_requested);
            assert_eq!(response.world_event_state_save_requested, 1);
            assert!(response.force_game_event_update_requested);
        }
        other => panic!("expected ok response, got {other:?}"),
    }
}
#[tokio::test]
async fn game_event_quest_complete_notify_reports_send_failed_like_cpp() {
    let (mut session, _, _) = make_session();
    let (command_tx, command_rx) = flume::bounded(1);
    drop(command_rx);
    session.set_game_event_quest_complete_sender_like_cpp(command_tx);

    let outcome = session.notify_game_event_quest_complete_like_cpp(77).await;

    assert_eq!(
        outcome,
        GameEventQuestCompleteClientOutcomeLikeCpp::SendFailed { quest_id: 77 }
    );
}
#[tokio::test]
async fn game_event_quest_complete_notify_reports_timeout_like_cpp() {
    let (mut session, _, _) = make_session();
    let (command_tx, command_rx) = flume::bounded(1);
    session.set_game_event_quest_complete_sender_like_cpp(command_tx);

    let outcome = session.notify_game_event_quest_complete_like_cpp(88).await;

    assert_eq!(
        outcome,
        GameEventQuestCompleteClientOutcomeLikeCpp::ResponseTimeout { quest_id: 88 }
    );
    drop(command_rx);
}
#[tokio::test]
async fn game_event_quest_complete_notify_reports_response_channel_closed_like_cpp() {
    let (mut session, _, _) = make_session();
    let (command_tx, command_rx) = flume::bounded(1);
    session.set_game_event_quest_complete_sender_like_cpp(command_tx);

    let closer = tokio::spawn(async move {
        let command = command_rx.recv_async().await.expect("command received");
        assert_eq!(command.quest_id, 99);
        drop(command.response_tx);
    });

    let outcome = session.notify_game_event_quest_complete_like_cpp(99).await;
    closer.await.expect("closer task completed");

    assert_eq!(
        outcome,
        GameEventQuestCompleteClientOutcomeLikeCpp::ResponseChannelClosed { quest_id: 99 }
    );
}
#[test]
fn reset_seasonal_clears_quest_v2_completed_bit_from_canonical_player_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 12_345);
    let canonical = shared_canonical_map_manager();
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SeasonalOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical seasonal map");
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 99);
    session.set_quest_v2_store(Arc::new(seasonal_quest_v2_store_like_cpp([(1001, 65)])));
    assert!(session.set_loaded_quest_completed_bit_like_cpp(65));

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(outcome.removed_quest_ids, vec![1001]);
    assert_eq!(outcome.completed_bit_cleared, 1);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 0);
    assert_eq!(outcome.completed_bit_skipped_zero_unique_bit, 0);
    assert_eq!(outcome.completed_bit_no_change_or_noop, 0);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert_eq!(session.seasonal_quest_bucket_like_cpp(7), None);
    let guard = canonical.lock().expect("canonical lock");
    let player = guard
        .find_map(571, 0)
        .expect("map")
        .map()
        .get_typed_player(player_guid)
        .expect("player");
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(0));
}
#[test]
fn reset_seasonal_missing_quest_v2_store_removes_without_inventing_bit_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 99);

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(outcome.removed_quest_ids, vec![1001]);
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 1);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert_eq!(session.seasonal_quest_bucket_like_cpp(7), None);
    assert!(session.represented_quest_completed_bits_like_cpp.is_empty());
}
#[test]
fn can_take_quest_rejects_completed_seasonal_bucket_quest_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest = seasonal_test_quest_template(12_345, -376, 9);
    session.seed_seasonal_quest_status_like_cpp(9, 12_345, 100);

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_allows_seasonal_when_only_other_event_bucket_has_quest_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest = seasonal_test_quest_template(12_345, -376, 9);
    session.seed_seasonal_quest_status_like_cpp(10, 12_345, 100);

    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_allows_seasonal_when_bucket_missing_or_empty_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest = seasonal_test_quest_template(12_345, -376, 9);

    assert!(session.can_take_quest(&quest));

    session.seed_empty_seasonal_event_bucket_like_cpp(9);
    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_allows_non_seasonal_even_when_same_bucket_has_quest_like_cpp() {
    let (mut session, _, _) = make_session();
    let quest = seasonal_test_quest_template(12_345, -101, 9);
    session.seed_seasonal_quest_status_like_cpp(9, 12_345, 100);

    assert!(session.can_take_quest(&quest));
}
#[test]
fn canonical_player_quest_rewarded_talent_points_follow_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_568);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "QuestTalentOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.add_represented_quest_reward_talent_points_like_cpp(10_001, 2));
    assert_eq!(
        session.represented_quest_rewarded_talent_points_like_cpp(),
        Some(2)
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(session.add_represented_quest_reward_talent_points_like_cpp(10_002, 3));
    assert_eq!(
        session.represented_quest_rewarded_talent_points_like_cpp(),
        Some(5)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(
        session.represented_quest_rewarded_talent_points_like_cpp(),
        None
    );
    assert!(!session.add_represented_quest_reward_talent_points_like_cpp(10_003, 7));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().quest_rewarded_talent_points
            }),
        Some(0)
    );
}
#[test]
fn canonical_access_requirement_uses_team_quest_reward_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 86);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessQuest".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        2,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.quest_done_a = 100;
    requirement.quest_done_h = 200;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("missing Horde quest abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );

    session.rewarded_quests.insert(200);
    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_access_requirement_quest_failed_text_sends_system_message_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 89);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessQuestText".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        2,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: "localized difficulty failure".to_string(),
            map_id: 631,
            difficulty_id: 3,
            lock_id: 77,
            reset_interval: 2,
            max_players: 25,
            flags: 0,
        },
    ])));
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.quest_done_h = 200;
    requirement.quest_failed_text = "Finish the attunement first.".to_string();
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("missing quest failed sysmessage"),
        ChatPkt {
            msg_type: ChatMsg::System,
            language: 0,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: "Finish the attunement first.".to_string(),
            virtual_realm: session.virtual_realm_address(),
        }
        .to_bytes()
    );
    assert_eq!(
        send_rx.try_recv().expect("missing quest failed abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn represented_request_vehicle_exit_accepts_control_seat_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 52);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("VehicleControlExitTester".to_string());
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_CONTROL);
    session.player_vehicle_seat_id_like_cpp = Some(1003);
    session.set_player_registry(Arc::clone(&registry));
    session.register_in_player_registry();

    assert!(session.represented_request_vehicle_exit_like_cpp());

    let info = registry.party_member(guid).expect("registered player");
    assert!(!info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 0);
}
#[test]
fn represented_adjacent_vehicle_seat_request_records_cpp_change_seat_plan() {
    let (mut session, _, _) = make_session();

    assert!(!session.represented_request_adjacent_vehicle_seat_like_cpp(false));
    assert!(
        session
            .represented_vehicle_seat_change_requests_like_cpp()
            .is_empty()
    );

    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    assert!(!session.represented_request_adjacent_vehicle_seat_like_cpp(true));
    assert!(
        session
            .represented_vehicle_seat_change_requests_like_cpp()
            .is_empty()
    );

    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_SWITCH);
    assert!(session.represented_request_adjacent_vehicle_seat_like_cpp(false));
    assert!(session.represented_request_adjacent_vehicle_seat_like_cpp(true));
    assert_eq!(
        session.represented_vehicle_seat_change_requests_like_cpp(),
        &[
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: -1,
                next: false,
            },
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: -1,
                next: true,
            },
        ]
    );
}
#[tokio::test]
async fn request_vehicle_prev_next_handlers_record_represented_change_seat_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_SWITCH);

    session
        .handle_request_vehicle_prev_seat(wow_packet::packets::vehicle::RequestVehiclePrevSeat)
        .await;
    session
        .handle_request_vehicle_next_seat(wow_packet::packets::vehicle::RequestVehicleNextSeat)
        .await;

    assert_eq!(
        session.represented_vehicle_seat_change_requests_like_cpp(),
        &[
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: -1,
                next: false,
            },
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: -1,
                next: true,
            },
        ],
        "C++ prev/next handlers call Player::ChangeSeat(-1, false/true) after CanSwitchFromSeat"
    );
}
#[test]
fn represented_request_vehicle_exit_rejects_non_exit_seat_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 51);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("VehicleExitRejectTester".to_string());
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    session.player_vehicle_seat_id_like_cpp = Some(1002);
    session.set_player_registry(Arc::clone(&registry));
    session.register_in_player_registry();

    assert!(!session.represented_request_vehicle_exit_like_cpp());

    assert_eq!(
        session.player_vehicle_seat_flags_like_cpp,
        Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK)
    );
    assert_eq!(session.player_vehicle_seat_id_like_cpp, Some(1002));
    let info = registry.party_member(guid).expect("registered player");
    assert!(info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 1002);
}
#[test]
fn player_homebind_update_request_preserves_wide_semantic_values_for_adapter() {
    let guid = ObjectGuid::create_player(1, 5007);
    let homebind = RepresentedHomebindLikeCpp {
        map_id: 571,
        area_id: 495,
        position: Position::new(11.0, 22.0, 33.0, 1.5),
    };

    let request =
        WorldSession::player_homebind_update_request_like_cpp(homebind, guid.counter() as u64);

    assert_eq!(
        request,
        wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            player_guid: guid.counter() as u64,
            map_id: 571,
            area_id: 495,
            x: 11.0,
            y: 22.0,
            z: 33.0,
            orientation: 1.5,
        }
    );

    let wide = WorldSession::player_homebind_update_request_like_cpp(
        RepresentedHomebindLikeCpp {
            map_id: u32::MAX - 2,
            area_id: u32::MAX - 1,
            position: Position::ZERO,
        },
        guid.counter() as u64,
    );
    assert!(matches!(
        wide,
        wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            map_id,
            area_id,
            ..
        } if map_id == u32::MAX - 2 && area_id == u32::MAX - 1
    ));
}
#[test]
fn xp_and_consumed_rest_state_share_one_semantic_request_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 5009);
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    let victim = test_creature_guid(80);
    install_tapped_xp_victim_like_cpp(&mut session, victim);
    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    let request =
        session.current_player_xp_persistence_request_like_cpp(false, true, guid.counter() as u64);

    assert!(!request.level_changed);
    assert_eq!(request.xp, 100);
    assert_eq!(request.rest.expect("consumed rest state").rest_bonus, 20.0);
}
#[test]
fn xp_and_zero_award_rest_state_normalization_share_semantic_request_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1C5);
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 0.5);
    let old_rest_bonus = session.represented_xp_rest_bonus_like_cpp();
    let old_rest_state = session.represented_xp_rest_state_like_cpp();
    let victim = test_creature_guid(0xE1C5);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));
    let rest_info_changed =
        session.represented_xp_rest_info_changed_since_like_cpp(old_rest_bonus, old_rest_state);
    let request = session.current_player_xp_persistence_request_like_cpp(
        false,
        rest_info_changed,
        guid.counter() as u64,
    );

    assert!(
        rest_info_changed,
        "state-only normalization must be persisted"
    );
    let rest = request.rest.expect("normalized rest state");
    assert_eq!(rest.rest_state, REST_STATE_NORMAL_LIKE_CPP);
    assert_eq!(rest.rest_bonus, 0.5);
}
#[test]
fn xp_and_sanitized_negative_rest_bonus_share_semantic_request_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1C7);
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, -0.5);
    let old_rest_bonus = session.represented_xp_rest_bonus_like_cpp();
    let old_rest_state = session.represented_xp_rest_state_like_cpp();
    let victim = test_creature_guid(0xE1C7);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
    let rest_info_changed =
        session.represented_xp_rest_info_changed_since_like_cpp(old_rest_bonus, old_rest_state);
    let request = session.current_player_xp_persistence_request_like_cpp(
        false,
        rest_info_changed,
        guid.counter() as u64,
    );

    assert!(
        rest_info_changed,
        "sanitizing a corrupt persisted float must not be lost on relog"
    );
    assert_eq!(request.rest.expect("sanitized rest state").rest_bonus, 0.0);
}
#[test]
fn questgiver_quest_list_leaves_level_fields_zero_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
    let mut first = test_quest_template(9_001);
    first.quest_level = 10;
    first.quest_max_scaling_level = 70;
    first.log_title = "First".into();
    let mut second = test_quest_template(9_002);
    second.quest_level = 11;
    second.quest_max_scaling_level = 80;
    second.log_title = "Second".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([first, second]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(777, 9_001));
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(777, 9_002));
    session.quest_store = Some(Arc::new(quest_store));

    assert!(session.use_represented_gameobject_questgiver_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::QuestgiverUseSource { gossip_id: 123 },
    ));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestListMessage)
    );
    assert_eq!(
        quest_list_level_fields_like_cpp(&bytes),
        vec![(9_001, 0, 0), (9_002, 0, 0)]
    );
}
#[test]
fn quest_giver_accept_missing_or_player_source_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(canonical);
    let quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_213)]);
    let missing_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 786, 33);
    let player_guid = ObjectGuid::create_player(1, 123);

    assert!(
        !session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            missing_guid,
            9_213,
            &quest_store,
        )
    );
    assert!(
        !session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            player_guid,
            9_213,
            &quest_store,
        )
    );
    assert!(session.player_quests.is_empty());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_reward_missing_or_player_source_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(canonical);
    let quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_217)]);
    let missing_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 789, 37);
    let player_guid = ObjectGuid::create_player(1, 123);

    assert!(
        !session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            missing_guid,
            9_217,
            &quest_store,
        )
    );
    assert!(
        !session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            player_guid,
            9_217,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_reward_money_crossing_cap_leaves_balance_unchanged_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 792, 40);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            player.unit_mut().set_faction(1);
        })
        .expect("live canonical Player fixture");
    add_canonical_test_creature(
        &canonical,
        source_guid,
        792,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );

    let quest_id = 9_233;
    let mut quest = test_quest_template(quest_id);
    quest.reward_money_difficulty = 2;
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    quest_store.ender_quests.insert(792, vec![quest_id]);
    session.quest_store = Some(Arc::new(quest_store));
    session.set_player_gold_like_cpp(MAX_MONEY_AMOUNT - 1);
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.statuses.insert(
                quest_id,
                crate::handlers::quest::PlayerQuestStatus {
                    quest_id,
                    status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: Vec::new(),
                    slot: 0,
                },
            );
        })
        .expect("canonical Player quest owner");

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            source_guid,
            quest_id,
            0,
            0,
        ))
        .await;

    assert_eq!(session.player_gold_like_cpp(), MAX_MONEY_AMOUNT - 1);
    assert_eq!(
        session
            .player_quest_gameplay_snapshot_like_cpp()
            .map(|quests| quests.rewarded_quest_ids.contains(&quest_id)),
        Some(true)
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_missing_source_rejects_before_mutation_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let missing_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 793, 41);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(5);
    let mut quest = test_quest_template(9_224);
    quest.reward_money_difficulty = 37;
    session.quest_store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        9_224,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_224,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            missing_guid,
            9_224,
            0,
            0,
        ))
        .await;

    assert_eq!(
        session.player_quests.get(&9_224).map(|quest| quest.status),
        Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&9_224));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_auto_complete_player_source_is_not_blocked_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    session.set_player_guid(Some(player_guid));
    session.set_loot_money_persistence_test_result_like_cpp(true);
    session.set_player_gold_like_cpp(5);
    let mut quest = test_quest_template(9_226);
    quest.flags = crate::handlers::quest::QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    session.quest_store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        9_226,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_226,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            9_226,
            0,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&9_226));
    assert!(session.rewarded_quests.contains(&9_226));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
