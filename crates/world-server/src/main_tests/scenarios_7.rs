//! Scenarios for [`super`], part 7.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_world_state_mgr_map_specific_null_map_is_unsupported_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 780,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::map_specific(
                780,
                0,
                [1],
            )],
            [],
        );

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(
        summary.update_world_states_map_specific_no_map_unsupported,
        1
    );
    assert_eq!(summary.update_world_states_global_message_represented, 0);
    assert_eq!(world_state_mgr.realm_value_like_cpp(780), 0);
}
#[test]
fn game_event_world_state_global_fanout_sends_update_to_active_players_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 777,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                777, 0,
            )],
            [],
        );
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx_a, send_rx_a) = flume::bounded(2);
    let (command_tx_a, _command_rx_a) = flume::bounded(1);
    let (send_tx_b, send_rx_b) = flume::bounded(2);
    let (command_tx_b, _command_rx_b) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7001, send_tx_a, command_tx_a);
    insert_player_registration_fixture_like_cpp(&registry, 7002, send_tx_b, command_tx_b);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id: 777,
        value: 1,
        hidden: false,
    }
    .to_bytes();
    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(summary.update_world_states_global_message_send_attempted, 2);
    assert_eq!(summary.update_world_states_global_message_send_queued, 2);
    assert_eq!(summary.update_world_states_global_message_send_failed, 0);
    assert_eq!(send_rx_a.try_recv().expect("player A update"), expected);
    assert_eq!(send_rx_b.try_recv().expect("player B update"), expected);
    assert!(send_rx_a.try_recv().is_err());
    assert!(send_rx_b.try_recv().is_err());
}
#[test]
fn game_event_world_state_global_fanout_skips_not_in_world_player_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (in_world_tx, in_world_rx) = flume::bounded(1);
    let (in_world_command_tx, _in_world_command_rx) = flume::bounded(1);
    let (not_in_world_tx, not_in_world_rx) = flume::bounded(1);
    let (not_in_world_command_tx, _not_in_world_command_rx) = flume::bounded(1);
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7901,
        in_world_tx,
        in_world_command_tx,
        true,
    );
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7902,
        not_in_world_tx,
        not_in_world_command_tx,
        false,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_realm_update_world_state_to_player_sessions_like_cpp(
        Some(&registry),
        782,
        1,
        false,
        &mut summary,
    );

    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id: 782,
        value: 1,
        hidden: false,
    }
    .to_bytes();
    assert_eq!(summary.update_world_states_global_message_send_attempted, 1);
    assert_eq!(summary.update_world_states_global_message_send_queued, 1);
    assert_eq!(summary.update_world_states_global_message_send_failed, 0);
    assert_eq!(
        summary.update_world_states_global_message_not_in_world_skipped,
        1
    );
    assert_eq!(
        in_world_rx.try_recv().expect("in-world player update"),
        expected
    );
    assert!(not_in_world_rx.try_recv().is_err());
}
#[test]
fn game_event_world_state_global_fanout_preserves_signed_value_and_wrapped_variable_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, _command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7003, send_tx, command_tx);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_realm_update_world_state_to_player_sessions_like_cpp(
        Some(&registry),
        -1,
        -42,
        false,
        &mut summary,
    );

    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id: u32::MAX,
        value: -42,
        hidden: false,
    }
    .to_bytes();
    assert_eq!(summary.update_world_states_global_message_send_attempted, 1);
    assert_eq!(summary.update_world_states_global_message_send_queued, 1);
    assert_eq!(
        send_rx.try_recv().expect("wrapped world-state update"),
        expected
    );
}
#[test]
fn game_event_world_state_realm_unchanged_does_not_fanout_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 778,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                778, 1,
            )],
            [],
        );
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, _command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7004, send_tx, command_tx);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    assert_eq!(summary.update_world_states_realm_unchanged_noop, 1);
    assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn game_event_world_state_realm_change_without_player_registry_is_counted_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 779,
        flags: 0,
    }]);
    let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(
        summary.update_world_states_global_message_registry_missing,
        1
    );
    assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
}
#[test]
fn game_event_world_state_map_specific_null_map_does_not_fanout_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 780,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::map_specific(
                780,
                0,
                [1],
            )],
            [],
        );
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, _command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7005, send_tx, command_tx);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    assert_eq!(
        summary.update_world_states_map_specific_no_map_unsupported,
        1
    );
    assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn game_event_world_state_global_fanout_counts_full_channel_failure_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 781,
        flags: 0,
    }]);
    let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (queued_tx, queued_rx) = flume::bounded(1);
    let (queued_command_tx, _queued_command_rx) = flume::bounded(1);
    let (full_tx, _full_rx) = flume::bounded(0);
    let (full_command_tx, _full_command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7006, queued_tx, queued_command_tx);
    insert_player_registration_fixture_like_cpp(&registry, 7007, full_tx, full_command_tx);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    assert_eq!(summary.update_world_states_global_message_send_attempted, 2);
    assert_eq!(summary.update_world_states_global_message_send_queued, 1);
    assert_eq!(summary.update_world_states_global_message_send_failed, 1);
    assert!(queued_rx.try_recv().is_ok());
}
#[test]
fn game_event_announce_start_order_before_spawn_and_stop_has_no_announce_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        3,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 2,
            description: "Darkmoon Faire".to_string(),
            announce: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = game_event_world_state_start_outcome_like_cpp(2);
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 3,
            state_before_raw: 0,
            state_after_raw: 0,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: false,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        },
    )];

    let actions = game_event_live_update_actions_like_cpp(&metadata, &outcome, false);

    assert_eq!(
        actions.first(),
        Some(&GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
            event_id: 2,
            description: "Darkmoon Faire".to_string(),
            description_len: "Darkmoon Faire".len(),
            announce: 1,
            config_event_announce: false,
        })
    );
    assert_eq!(
        actions.get(1),
        Some(&GameEventLiveUpdateActionLikeCpp::Spawn(2))
    );
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(
                action,
                GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
            ))
            .count(),
        1
    );
    assert!(matches!(
        actions.iter().rev().take(8).last(),
        Some(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
            event_id: 3,
            activate: false
        })
    ));
}
#[test]
fn game_event_announce_gating_matches_cpp_config_like_cpp() {
    let mut event = spawn_store_loader::GameEventDataLikeCpp {
        event_id: 1,
        description: "config gated".to_string(),
        ..spawn_store_loader::GameEventDataLikeCpp::default()
    };
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    event.announce = 1;
    let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
    assert!(matches!(
        game_event_live_update_actions_like_cpp(&metadata, &outcome, false).first(),
        Some(GameEventLiveUpdateActionLikeCpp::AnnounceEvent { announce: 1, .. })
    ));

    event.announce = 2;
    let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
    assert!(
        !game_event_live_update_actions_like_cpp(&metadata, &outcome, false)
            .iter()
            .any(|action| matches!(
                action,
                GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
            ))
    );
    assert!(matches!(
        game_event_live_update_actions_like_cpp(&metadata, &outcome, true).first(),
        Some(GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
            announce: 2,
            config_event_announce: true,
            ..
        })
    ));

    for announce in [0_u8, 3_u8] {
        event.announce = announce;
        let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
        assert!(
            !game_event_live_update_actions_like_cpp(&metadata, &outcome, true)
                .iter()
                .any(|action| matches!(
                    action,
                    GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
                ))
        );
    }
}
#[test]
fn game_event_announce_consumption_fans_out_system_chat_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            description: "Darkmoon Faire".to_string(),
            announce: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_world_state_start_outcome_like_cpp(1);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx_a, send_rx_a) = flume::bounded(2);
    let (command_tx_a, _command_rx_a) = flume::bounded(1);
    let (send_tx_b, send_rx_b) = flume::bounded(2);
    let (command_tx_b, _command_rx_b) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7101, send_tx_a, command_tx_a);
    insert_player_registration_fixture_like_cpp(&registry, 7102, send_tx_b, command_tx_b);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        Some(&registry),
        &[1],
        &outcome,
        false,
    );

    let expected_packet = ChatPkt {
        msg_type: ChatMsg::System,
        language: 0,
        sender_guid: ObjectGuid::EMPTY,
        sender_name: String::new(),
        target_guid: ObjectGuid::EMPTY,
        target_name: String::new(),
        prefix: String::new(),
        channel: String::new(),
        text: "|cffff0000[Event Message]: Darkmoon Faire|r".to_string(),
        virtual_realm: 0,
    };
    let mut expected_payload = wow_packet::world_packet::WorldPacket::new_empty();
    expected_packet.write(&mut expected_payload);
    assert_eq!(
        expected_payload.data()[0],
        0x00,
        "CHAT_MSG_SYSTEM must be 0x00 on wire"
    );
    assert_eq!(&expected_payload.data()[1..5], &[0x00, 0x00, 0x00, 0x00]);
    let expected = expected_packet.to_bytes();

    assert_eq!(summary.announce_event_actions, 1);
    assert_eq!(
        summary.announce_event_description_len_total,
        "Darkmoon Faire".len()
    );
    assert_eq!(summary.announce_event_world_text_represented, 1);
    assert_eq!(summary.announce_event_localization_unrepresented, 1);
    assert_eq!(summary.announce_event_in_world_filter_unrepresented, 0);
    assert_eq!(summary.announce_event_not_in_world_skipped, 0);
    assert_eq!(summary.announce_event_lines, 1);
    assert_eq!(summary.announce_event_send_attempted, 2);
    assert_eq!(summary.announce_event_send_queued, 2);
    assert_eq!(summary.announce_event_send_failed, 0);
    assert_eq!(summary.announce_event_world_text_unimplemented, 0);
    assert_eq!(summary.announce_event_session_fanout_unimplemented, 0);
    let received_a = send_rx_a.try_recv().expect("player A packet");
    let received_b = send_rx_b.try_recv().expect("player B packet");
    let payload_offset = 2; // ServerPacket::to_bytes prepends the u16 opcode.
    assert_eq!(
        received_a[payload_offset], 0x00,
        "received CHAT_MSG_SYSTEM must be 0x00 on wire"
    );
    assert_eq!(
        &received_a[payload_offset + 1..payload_offset + 5],
        &[0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        received_b[payload_offset], 0x00,
        "received CHAT_MSG_SYSTEM must be 0x00 on wire"
    );
    assert_eq!(
        &received_b[payload_offset + 1..payload_offset + 5],
        &[0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(received_a, expected);
    assert_eq!(received_b, expected);
    assert!(send_rx_a.try_recv().is_err());
    assert!(send_rx_b.try_recv().is_err());
    assert_eq!(summary.spawn_actions, 1);
}
#[test]
fn game_event_announce_fanout_skips_not_in_world_player_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (in_world_tx, in_world_rx) = flume::bounded(1);
    let (in_world_command_tx, _in_world_command_rx) = flume::bounded(1);
    let (not_in_world_tx, not_in_world_rx) = flume::bounded(1);
    let (not_in_world_command_tx, _not_in_world_command_rx) = flume::bounded(1);
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7903,
        in_world_tx,
        in_world_command_tx,
        true,
    );
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7904,
        not_in_world_tx,
        not_in_world_command_tx,
        false,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_game_event_announcement_to_player_sessions_like_cpp(
        Some(&registry),
        "Darkmoon Faire",
        &mut summary,
    );

    let expected = ChatPkt {
        msg_type: ChatMsg::System,
        language: 0,
        sender_guid: ObjectGuid::EMPTY,
        sender_name: String::new(),
        target_guid: ObjectGuid::EMPTY,
        target_name: String::new(),
        prefix: String::new(),
        channel: String::new(),
        text: "|cffff0000[Event Message]: Darkmoon Faire|r".to_string(),
        virtual_realm: 0,
    }
    .to_bytes();
    assert_eq!(summary.announce_event_world_text_represented, 1);
    assert_eq!(summary.announce_event_localization_unrepresented, 1);
    assert_eq!(summary.announce_event_in_world_filter_unrepresented, 0);
    assert_eq!(summary.announce_event_not_in_world_skipped, 1);
    assert_eq!(summary.announce_event_lines, 1);
    assert_eq!(summary.announce_event_send_attempted, 1);
    assert_eq!(summary.announce_event_send_queued, 1);
    assert_eq!(summary.announce_event_send_failed, 0);
    assert_eq!(
        in_world_rx.try_recv().expect("in-world player chat"),
        expected
    );
    assert!(not_in_world_rx.try_recv().is_err());
}
#[test]
fn game_event_announce_missing_registry_counts_gap_without_panic_like_cpp() {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_game_event_announcement_to_player_sessions_like_cpp(
        None,
        "Love is in the Air",
        &mut summary,
    );

    assert_eq!(summary.announce_event_world_text_represented, 1);
    assert_eq!(summary.announce_event_localization_unrepresented, 1);
    assert_eq!(summary.announce_event_registry_missing, 1);
    assert_eq!(summary.announce_event_lines, 1);
    assert_eq!(summary.announce_event_send_attempted, 0);
    assert_eq!(summary.announce_event_send_queued, 0);
    assert_eq!(summary.announce_event_send_failed, 0);
}
#[test]
fn game_event_announce_newline_split_after_fallback_format_like_cpp() {
    assert_eq!(
        game_event_announcement_lines_like_cpp(""),
        vec!["|cffff0000[Event Message]: |r".to_string()]
    );
    assert_eq!(
        game_event_announcement_lines_like_cpp("\n\n"),
        vec!["|cffff0000[Event Message]: ".to_string(), "|r".to_string(),]
    );
    assert_eq!(
        game_event_announcement_lines_like_cpp("A\n\nB"),
        vec![
            "|cffff0000[Event Message]: A".to_string(),
            "B|r".to_string(),
        ]
    );
}
#[test]
fn game_event_smart_ai_game_event_seasonal_start_stop_order_matches_cpp_live_update_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        3,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 2,
            start: 100,
            occurence: 10,
            state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 1_350,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![2],
        queued_deactivation_event_ids: vec![3],
        start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id: 2,
                state_before_raw: 0,
                state_after_raw: 0,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            },
        )],
        stop_outcomes: vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
            spawn_store_loader::GameEventStopSummaryLikeCpp {
                event_id: 3,
                state_before_raw: 0,
                state_after_raw: 0,
                active_removed: true,
                active_was_present: true,
                unapply_event_requested: true,
                serverwide: true,
                condition_reset_requested: false,
                delete_world_event_state_requested: false,
                delete_condition_saves_requested: false,
            },
        )],
        negative_spawn_event_ids: vec![-1],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    };

    assert_eq!(
        game_event_live_update_actions_like_cpp(&metadata, &outcome, false),
        vec![
            GameEventLiveUpdateActionLikeCpp::Spawn(-1),
            GameEventLiveUpdateActionLikeCpp::Spawn(2),
            GameEventLiveUpdateActionLikeCpp::Unspawn(-2),
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id: 2 },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id: 2,
                event_start_time: 1_300,
            },
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::Unspawn(3),
            GameEventLiveUpdateActionLikeCpp::Spawn(-3),
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id: 3 },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                event_id: 3,
                activate: false,
            },
        ]
    );
}
#[test]
fn game_event_smart_ai_consume_no_maps_missing_event_noops_and_counts_action_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(0, &[]);
    let outcome = spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 650,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![7],
        queued_deactivation_event_ids: vec![],
        start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id: 7,
                state_before_raw: 0,
                state_after_raw: 0,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            },
        )],
        stop_outcomes: vec![],
        negative_spawn_event_ids: vec![],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    };

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[7],
        &outcome,
        false,
    );

    assert_eq!(summary.run_smart_ai_actions, 1);
    assert_eq!(summary.run_smart_ai_maps_visited, 0);
    assert_eq!(summary.run_smart_ai_creature_candidates, 0);
    assert_eq!(summary.run_smart_ai_gameobject_candidates, 0);
    assert_eq!(summary.run_smart_ai_script_dispatch_unrepresented, 0);
}
