// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! client_state capability handler tests.

use super::*;

#[tokio::test]
async fn set_action_bar_toggles_updates_active_player_field_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 9001);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 3.0, 0.0));

    session
        .handle_set_action_bar_toggles(WorldPacket::from_bytes(&[0x2d]))
        .await;

    assert_eq!(session.active_player_multi_action_bars_like_cpp(), 0x2d);
    let sent = send_rx.try_recv().expect("VALUES update packet");
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::UpdateObject as u16
    );
}

#[tokio::test]
async fn set_action_bar_toggles_short_packet_does_not_mutate_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 9002)));
    session
        .handle_set_action_bar_toggles(WorldPacket::from_bytes(&[]))
        .await;

    assert_eq!(session.active_player_multi_action_bars_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_advanced_combat_logging_sets_and_clears_player_state_like_cpp() {
    let (mut session, _send_rx) = make_session();

    let mut enable = WorldPacket::new_empty();
    enable.write_bit(true);
    enable.flush_bits();
    enable.reset_read();
    session.handle_set_advanced_combat_logging(enable).await;
    assert!(session.represented_advanced_combat_logging_enabled_like_cpp());

    let mut disable = WorldPacket::new_empty();
    disable.write_bit(false);
    disable.flush_bits();
    disable.reset_read();
    session.handle_set_advanced_combat_logging(disable).await;
    assert!(!session.represented_advanced_combat_logging_enabled_like_cpp());
}

#[tokio::test]
async fn set_advanced_combat_logging_short_packet_does_not_change_state_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.represented_set_advanced_combat_logging_like_cpp(true);

    session
        .handle_set_advanced_combat_logging(WorldPacket::from_bytes(&[]))
        .await;

    assert!(session.represented_advanced_combat_logging_enabled_like_cpp());
}

#[tokio::test]
async fn set_currency_flags_updates_existing_currency_and_sends_setup_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        wow_data::CurrencyTypesEntry {
            max_qty: 200,
            max_earnable_per_week: 50,
            flags: wow_constants::CurrencyTypesFlags::TRACK_QUANTITY,
            flags_b: wow_constants::CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
            ..currency_entry(395)
        },
    ])));
    session.set_player_currencies_like_cpp(HashMap::from([(
        395,
        crate::session::PlayerCurrency {
            state: crate::session::PlayerCurrencyState::Unchanged,
            quantity: 123,
            weekly_quantity: 20,
            tracked_quantity: 7,
            increased_cap_quantity: 0,
            earned_quantity: 300,
            flags: 0,
        },
    )]));

    let mut request = WorldPacket::new_empty();
    request.write_uint32(395);
    request.write_uint8(0x1f);
    request.reset_read();
    session.handle_set_currency_flags(request).await;

    let currencies = session.player_currencies_like_cpp().unwrap();
    let currency = currencies.get(&395).unwrap();
    assert_eq!(currency.flags, 0x1f);
    assert_eq!(currency.state, crate::session::PlayerCurrencyState::Changed);

    let sent = send_rx.try_recv().expect("SMSG_SETUP_CURRENCY");
    let mut setup = WorldPacket::from_bytes(&sent);
    assert_eq!(setup.server_opcode(), Some(ServerOpcodes::SetupCurrency));
    setup.skip_opcode();
    assert_eq!(setup.read_uint32().unwrap(), 1);
    assert_eq!(setup.read_int32().unwrap(), 395);
    assert_eq!(setup.read_int32().unwrap(), 123);
    assert!(setup.read_bit().unwrap());
    assert!(setup.read_bit().unwrap());
    assert!(setup.read_bit().unwrap());
    assert!(setup.read_bit().unwrap());
    assert!(setup.read_bit().unwrap());
    assert!(!setup.read_bit().unwrap());
    assert!(!setup.read_bit().unwrap());
    assert_eq!(setup.read_bits(5).unwrap(), 0x0c);
    assert_eq!(setup.read_uint32().unwrap(), 20);
    assert_eq!(setup.read_uint32().unwrap(), 50);
    assert_eq!(setup.read_uint32().unwrap(), 7);
    assert_eq!(setup.read_int32().unwrap(), 200);
    assert_eq!(setup.read_int32().unwrap(), 300);
}

#[tokio::test]
async fn set_currency_flags_missing_player_currency_still_replays_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(395),
    ])));

    let mut request = WorldPacket::new_empty();
    request.write_uint32(395);
    request.write_uint8(0x1f);
    request.reset_read();
    session.handle_set_currency_flags(request).await;

    assert!(
        session
            .player_currencies_like_cpp()
            .unwrap()
            .get(&395)
            .is_none()
    );
    let sent = send_rx.try_recv().expect("C++ still calls SendCurrencies");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::SetupCurrency)
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_currency_flags_short_packet_does_not_send_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_set_currency_flags(WorldPacket::from_bytes(&[0x01, 0x00]))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_ammo_is_silent_like_cpp_debug_only_handler() {
    let (mut session, send_rx) = make_session();

    session.handle_set_ammo(WorldPacket::new_empty()).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_game_event_debug_view_state_is_silent_like_cpp_debug_only_handler() {
    let (mut session, send_rx) = make_session();

    session
        .handle_set_game_event_debug_view_state(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn add_battlenet_friend_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_add_battlenet_friend(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_insert_items_left_to_right_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_set_insert_items_left_to_right(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn client_telemetry_null_family_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    for _ in 0..5 {
        session
            .handle_client_telemetry_null_like_cpp(WorldPacket::new_empty())
            .await;
    }

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn unhandled_client_null_family_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    for _ in 0..5 {
        session
            .handle_unhandled_client_null_like_cpp(WorldPacket::new_empty())
            .await;
    }

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn showing_helm_and_cloak_are_silent_like_cpp_debug_only_handlers() {
    let (mut session, send_rx) = make_session();

    session.handle_showing_helm(WorldPacket::new_empty()).await;
    session.handle_showing_cloak(WorldPacket::new_empty()).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn loading_screen_notify_is_silent_like_cpp_todo_handler() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(571);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    session.handle_loading_screen_notify(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn violence_level_is_silent_like_cpp_todo_handler() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(2);
    pkt.reset_read();

    session.handle_violence_level(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn override_screen_flash_is_handle_null_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_override_screen_flash(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn queued_messages_end_is_handle_null_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_queued_messages_end(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn get_account_character_list_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_get_account_character_list(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn get_account_notifications_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_get_account_notifications(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn report_client_variables_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_report_client_variables(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn report_frozen_while_loading_map_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_report_frozen_while_loading_map(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn log_streaming_error_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_log_streaming_error(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn complete_movie_clears_active_movie_and_records_script_hook_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_complete_movie(WorldPacket::new_empty())
        .await;
    assert_eq!(session.represented_movie_like_cpp(), None);
    assert!(
        session
            .represented_movie_complete_events_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());

    session.set_represented_movie_like_cpp_for_test(Some(177));
    session
        .handle_complete_movie(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_movie_like_cpp(), None);
    assert_eq!(session.represented_movie_complete_events_like_cpp(), &[177]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn complete_cinematic_clears_active_cinematic_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_complete_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(session.represented_cinematic_like_cpp(), None);
    assert!(
        session
            .represented_cinematic_end_events_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());

    session.set_represented_cinematic_like_cpp_for_test(Some(444));
    session
        .handle_complete_cinematic(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_cinematic_like_cpp(), None);
    assert_eq!(session.represented_cinematic_end_events_like_cpp(), &[444]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn next_cinematic_camera_advances_active_camera_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_next_cinematic_camera(WorldPacket::new_empty())
        .await;
    assert!(
        session
            .represented_cinematic_next_camera_events_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());

    session.set_cinematic_sequences_store(Arc::new(
        wow_data::CinematicSequencesStore::from_entries([wow_data::CinematicSequencesEntry {
            id: 444,
            sound_id: 0,
            camera: [11, 22, 0, 33, 0, 0, 0, 0],
        }]),
    ));
    assert!(session.use_represented_gameobject_camera_like_cpp(
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 8),
        ObjectGuid::create_player(1, 99),
        wow_entities::CameraUseSource {
            cinematic_id: 444,
            event_id: 0,
        },
    ));
    let _ = send_rx.try_recv().expect("TriggerCinematic sent");
    assert_eq!(session.represented_cinematic_like_cpp(), Some(444));
    assert_eq!(session.represented_cinematic_camera_index_like_cpp(), -1);

    session
        .handle_next_cinematic_camera(WorldPacket::new_empty())
        .await;
    session
        .handle_next_cinematic_camera(WorldPacket::new_empty())
        .await;
    session
        .handle_next_cinematic_camera(WorldPacket::new_empty())
        .await;
    session
        .handle_next_cinematic_camera(WorldPacket::new_empty())
        .await;
    assert_eq!(
        session.represented_cinematic_next_camera_events_like_cpp(),
        &[11, 22, 33]
    );
    assert_eq!(session.represented_cinematic_camera_index_like_cpp(), 3);
    assert!(send_rx.try_recv().is_err());

    for _ in 0..8 {
        session
            .handle_next_cinematic_camera(WorldPacket::new_empty())
            .await;
    }
    assert_eq!(
        session.represented_cinematic_next_camera_events_like_cpp(),
        &[11, 22, 33]
    );
}

#[tokio::test]
async fn additional_status_unhandled_null_family_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_logout_instant(WorldPacket::new_empty())
        .await;
    session
        .handle_spawn_tracking_update(WorldPacket::new_empty())
        .await;
    session
        .handle_time_adjustment_response(WorldPacket::new_empty())
        .await;
    session
        .handle_update_area_trigger_visual(WorldPacket::new_empty())
        .await;
    session
        .handle_update_spell_visual(WorldPacket::new_empty())
        .await;
    session.handle_used_follow(WorldPacket::new_empty()).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn unhandled_movement_null_family_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    for _ in 0..19 {
        session
            .handle_unhandled_client_null_like_cpp(WorldPacket::new_empty())
            .await;
    }

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn report_keybinding_execution_counts_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_report_keybinding_execution_counts(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn query_countdown_timer_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_request_countdown_timer(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}
