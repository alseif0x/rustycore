//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn process_pending_ticks_default_not_ready_gameobject_to_ready_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let chair_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 78);
    let camera_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 79);
    let chest_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 80);
    for (guid, go_type) in [
        (chair_guid, wow_entities::GAMEOBJECT_TYPE_CHAIR),
        (camera_guid, wow_entities::GAMEOBJECT_TYPE_CAMERA),
        (chest_guid, wow_entities::GAMEOBJECT_TYPE_CHEST),
    ] {
        let state = session
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.go_type = Some(go_type as u8);
        state.loot_state = Some(wow_entities::LootState::NotReady);
    }

    session.process_pending().await;

    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&chair_guid)
            .unwrap()
            .loot_state,
        Some(wow_entities::LootState::Ready)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&camera_guid)
            .unwrap()
            .loot_state,
        Some(wow_entities::LootState::Ready)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&chest_guid)
            .unwrap()
            .loot_state,
        Some(wow_entities::LootState::NotReady)
    );
}
#[test]
fn gameobject_use_chair_picks_nearest_free_slot_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 17);

    assert!(session.use_represented_gameobject_chair_like_cpp(
        gameobject_guid,
        player_guid,
        Position::new(0.0, 1.8, 0.0, 0.0),
        Position::ZERO,
        2.0,
        wow_entities::ChairUseSource {
            chair_slots: 3,
            chair_height: 2,
            triggered_event_id: 55,
        },
    ));

    assert_eq!(session.represented_gameobject_use_effects.len(), 2);
    match session.represented_gameobject_use_effects[0] {
        RepresentedGameObjectUseEffect::ChairUsed {
            gameobject_guid: effect_gameobject_guid,
            player_guid: effect_player_guid,
            slot,
            teleport_position,
            stand_state,
        } => {
            assert_eq!(effect_gameobject_guid, gameobject_guid);
            assert_eq!(effect_player_guid, player_guid);
            assert_eq!(slot, 2);
            assert!((teleport_position.x - 0.0).abs() < 0.001);
            assert!((teleport_position.y - 2.0).abs() < 0.001);
            assert!((teleport_position.z - 0.0).abs() < 0.001);
            assert!((teleport_position.orientation - 0.0).abs() < 0.001);
            assert_eq!(stand_state, 6);
        }
        other => panic!("unexpected chair effect: {other:?}"),
    }
    assert_eq!(
        session.represented_gameobject_use_effects[1],
        RepresentedGameObjectUseEffect::TriggerGameEvent {
            gameobject_guid,
            player_guid,
            event_id: 55,
        }
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap()
            .chair_slots[2],
        Some(player_guid)
    );
    let session_position = session.player_position_like_cpp().unwrap();
    assert!((session_position.x - 0.0).abs() < 0.001);
    assert!((session_position.y - 2.0).abs() < 0.001);
    assert!((session_position.z - 0.0).abs() < 0.001);
    assert!((session_position.orientation - 0.0).abs() < 0.001);
    assert_eq!(
        session.player_stand_state_like_cpp(),
        UnitStandStateType::SitHighChair
    );
}
#[test]
fn gameobject_use_chair_rejects_when_all_represented_slots_are_occupied() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let other_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .chair_slots = vec![Some(other_guid)];

    assert!(!session.use_represented_gameobject_chair_like_cpp(
        gameobject_guid,
        player_guid,
        Position::ZERO,
        Position::ZERO,
        1.0,
        wow_entities::ChairUseSource {
            chair_slots: 1,
            chair_height: 0,
            triggered_event_id: 0,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::ChairNoFreeSlot {
            gameobject_guid,
            player_guid,
        }]
    );
}
#[test]
fn gameobject_use_barber_chair_records_ui_teleport_and_stand_state_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 22);
    let gameobject_position = Position::new(1.0, 2.0, 3.0, 4.0);

    assert!(session.use_represented_gameobject_barber_chair_like_cpp(
        gameobject_guid,
        player_guid,
        gameobject_position,
        wow_entities::BarberChairUseSource {
            chair_height: 2,
            sit_anim_kit: 55,
            customization_scope: 7,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::BarberChairUsed {
            gameobject_guid,
            player_guid,
            customization_scope: 7,
            teleport_position: gameobject_position,
            stand_state: 6,
            sit_anim_kit: 55,
        }]
    );
    let mut expected = (ServerOpcodes::EnableBarberShop as u16)
        .to_le_bytes()
        .to_vec();
    expected.push(7);
    assert_eq!(send_rx.try_recv().unwrap(), expected);
    assert_eq!(
        session.player_position_like_cpp(),
        Some(gameobject_position)
    );
    assert_eq!(
        session.player_stand_state_like_cpp(),
        UnitStandStateType::SitHighChair
    );
}
#[test]
fn gameobject_use_ui_link_records_interaction_type_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);

    assert!(session.use_represented_gameobject_ui_link_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::UiLinkUseSource { ui_link_type: 2 },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::UiLinkOpened {
            gameobject_guid,
            player_guid,
            ui_link_type: 2,
            interaction_type: 40,
        }]
    );
    let mut expected = (ServerOpcodes::GameObjectInteraction as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    expected.extend_from_slice(&40_i32.to_le_bytes());
    assert_eq!(send_rx.try_recv().unwrap(), expected);

    session.represented_gameobject_use_effects.clear();
    assert!(session.use_represented_gameobject_ui_link_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::UiLinkUseSource { ui_link_type: 99 },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::UiLinkOpened {
            gameobject_guid,
            player_guid,
            ui_link_type: 99,
            interaction_type: 0,
        }]
    );
}
#[test]
fn gameobject_use_item_forge_records_condition_checked_noop_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 24);

    assert!(session.use_represented_gameobject_item_forge_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::ItemForgeUseSource {
            condition_id: 77,
            forge_type: 4,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::ItemForgeUsed {
            gameobject_guid,
            player_guid,
            condition_id: 77,
            forge_type: 4,
        }]
    );
}
#[test]
fn gameobject_use_capture_point_records_assault_hook_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 25);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_AB_LIKE_CPP);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .position = Some(Position::new(12.5, 34.25, 56.0, 1.0));

    assert!(session.use_represented_gameobject_capture_point_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CapturePointUseSource {
            capture_time_ms: 60_000,
            world_state_id: 123,
            contested_event_horde: 456,
            contested_event_alliance: 789,
            ..Default::default()
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::CapturePointAssaultAi {
                gameobject_guid,
                player_guid,
                handled: false,
            },
            RepresentedGameObjectUseEffect::CapturePointAssaultRequested {
                gameobject_guid,
                player_guid,
                capture_time_ms: 60_000,
                world_state_id: 123,
                contested_event_horde: 456,
                contested_event_alliance: 789,
            },
            RepresentedGameObjectUseEffect::CapturePointUpdated {
                gameobject_guid,
                state: RepresentedCapturePointStateLikeCpp::ContestedAlliance,
                broadcast_text_id: 0,
                event_id: 789,
                world_state_id: 123,
                spell_visual_id: 0,
                custom_anim: 2,
                assault_timer_ms: 60_000,
            }
        ]
    );
    let packet = send_rx.try_recv().unwrap();
    assert_eq!(
        packet[0..2],
        (ServerOpcodes::UpdateCapturePoint as u16).to_le_bytes()
    );
    assert_eq!(&packet[2..18], &gameobject_guid.to_raw_bytes());
    assert_eq!(&packet[18..22], &12.5_f32.to_le_bytes());
    assert_eq!(&packet[22..26], &34.25_f32.to_le_bytes());
    assert_eq!(packet[26], 3);
    assert_eq!(&packet[27..31], &60_000_u32.to_le_bytes());
    assert_eq!(&packet[31..35], &60_000_u32.to_le_bytes());
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.capture_point_state,
        Some(RepresentedCapturePointStateLikeCpp::ContestedAlliance)
    );
    assert!(state.capture_point_assault_until.is_some());
}
#[test]
fn gameobject_use_capture_point_requires_battleground_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 42);

    assert!(!session.use_represented_gameobject_capture_point_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CapturePointUseSource {
            capture_time_ms: 60_000,
            world_state_id: 123,
            contested_event_horde: 456,
            contested_event_alliance: 789,
            ..Default::default()
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::CapturePointAssaultAi {
            gameobject_guid,
            player_guid,
            handled: false,
        }]
    );
}
#[test]
fn gameobject_use_capture_point_ai_can_handle_before_battleground_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 44);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .capture_point_assault_ai_returns_true = true;

    assert!(session.use_represented_gameobject_capture_point_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CapturePointUseSource {
            capture_time_ms: 60_000,
            world_state_id: 123,
            contested_event_horde: 456,
            contested_event_alliance: 789,
            ..Default::default()
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::CapturePointAssaultAi {
            gameobject_guid,
            player_guid,
            handled: true,
        }]
    );
}
#[test]
fn gameobject_use_capture_point_respects_team_state_gate_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 43);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_AB_LIKE_CPP);
    session.player_race = 1;
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .capture_point_state = Some(RepresentedCapturePointStateLikeCpp::AllianceCaptured);

    assert!(!session.use_represented_gameobject_capture_point_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CapturePointUseSource {
            capture_time_ms: 60_000,
            world_state_id: 123,
            contested_event_horde: 456,
            contested_event_alliance: 789,
            ..Default::default()
        },
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());

    session.player_race = 2;
    assert!(session.use_represented_gameobject_capture_point_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CapturePointUseSource {
            capture_time_ms: 60_000,
            world_state_id: 123,
            contested_event_horde: 456,
            contested_event_alliance: 789,
            ..Default::default()
        },
    ));
}
#[test]
fn gameobject_use_flagstand_records_battleground_click_hook_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 26);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);

    assert!(session.use_represented_gameobject_flagstand_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::FlagStandUseSource {
            pickup_spell_id: 11,
            return_aura_id: 33,
            return_spell_id: 44,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RemoveStealthOrInvisibilityAuras {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::BattlegroundFlagStandClicked {
                gameobject_guid,
                player_guid,
                pickup_spell_id: 11,
                return_aura_id: 33,
                return_spell_id: 44,
            }
        ]
    );
}
#[test]
fn gameobject_use_flagstand_rejects_vehicle_before_bg_click_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 40);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);
    session.player_vehicle_seat_flags_like_cpp = Some(0);

    assert!(!session.use_represented_gameobject_flagstand_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::FlagStandUseSource {
            pickup_spell_id: 11,
            return_aura_id: 33,
            return_spell_id: 44,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Vehicle,
            }
        ]
    );
}
#[test]
fn gameobject_use_flagstand_rejects_missing_battleground_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 42);

    assert!(!session.use_represented_gameobject_flagstand_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::FlagStandUseSource {
            pickup_spell_id: 11,
            return_aura_id: 33,
            return_spell_id: 44,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::NotInBattleground,
            }
        ]
    );
}
#[test]
fn gameobject_use_flagstand_removes_stealth_and_invisibility_auras_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 41);
    session.set_player_guid(Some(player_guid));
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);

    session.apply_aura(1001, player_guid, 30_000, 0).unwrap();
    session.apply_aura(1002, player_guid, 30_000, 0).unwrap();
    session
        .visible_auras
        .get_mut(&0)
        .unwrap()
        .represented_effect = Some(RepresentedAuraEffectLikeCpp::Stealth);
    session
        .visible_auras
        .get_mut(&1)
        .unwrap()
        .represented_effect = Some(RepresentedAuraEffectLikeCpp::Invisibility);
    while send_rx.try_recv().is_ok() {}

    assert!(session.use_represented_gameobject_flagstand_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::FlagStandUseSource {
            pickup_spell_id: 11,
            return_aura_id: 33,
            return_spell_id: 44,
        },
    ));

    assert!(session.visible_auras.values().all(|aura| !matches!(
        aura.represented_effect,
        Some(RepresentedAuraEffectLikeCpp::Stealth)
            | Some(RepresentedAuraEffectLikeCpp::Invisibility)
    )));
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
}
#[test]
fn gameobject_use_flagdrop_triggers_event_and_delete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 27);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);
    let map_id = session.player_map_id_like_cpp();

    assert!(session.use_represented_gameobject_flagdrop_like_cpp(
        gameobject_guid,
        player_guid,
        179785,
        wow_entities::FlagDropUseSource {
            event_id: 22,
            pickup_spell_id: 33,
            expire_duration_ms: 44,
        },
    ));
    let mut expected = (ServerOpcodes::GameObjectDespawn as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    assert_eq!(send_rx.try_recv().unwrap(), expected);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::update::UpdateObject::destroy_objects(vec![gameobject_guid], map_id)
            .to_bytes()
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RemoveStealthOrInvisibilityAuras {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::BattlegroundFlagDropClicked {
                gameobject_guid,
                player_guid,
                gameobject_entry: 179785,
                click_target: BattlegroundFlagDropClickTarget::WarsongGulch,
                event_id: 22,
                pickup_spell_id: 33,
                expire_duration_ms: 44,
            },
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 22,
            },
            RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid },
        ]
    );
}
#[test]
fn gameobject_use_flagdrop_requires_matching_battleground_type_for_click_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 43);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_EY_LIKE_CPP);

    assert!(session.use_represented_gameobject_flagdrop_like_cpp(
        gameobject_guid,
        player_guid,
        179785,
        wow_entities::FlagDropUseSource {
            event_id: 22,
            pickup_spell_id: 33,
            expire_duration_ms: 44,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RemoveStealthOrInvisibilityAuras {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::BattlegroundFlagDropClicked {
                gameobject_guid,
                player_guid,
                gameobject_entry: 179785,
                click_target: BattlegroundFlagDropClickTarget::None,
                event_id: 22,
                pickup_spell_id: 33,
                expire_duration_ms: 44,
            },
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 22,
            },
            RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid },
        ]
    );
}
#[test]
fn gameobject_use_flagdrop_rejects_missing_battleground_before_delete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 44);

    assert!(!session.use_represented_gameobject_flagdrop_like_cpp(
        gameobject_guid,
        player_guid,
        179785,
        wow_entities::FlagDropUseSource {
            event_id: 22,
            pickup_spell_id: 33,
            expire_duration_ms: 44,
        },
    ));
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::NotInBattleground,
            }
        ]
    );
}
#[test]
fn gameobject_use_flagdrop_rejects_vehicle_before_delete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 41);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);
    session.player_vehicle_seat_flags_like_cpp = Some(0);

    assert!(!session.use_represented_gameobject_flagdrop_like_cpp(
        gameobject_guid,
        player_guid,
        179785,
        wow_entities::FlagDropUseSource {
            event_id: 22,
            pickup_spell_id: 33,
            expire_duration_ms: 44,
        },
    ));
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Vehicle,
            }
        ]
    );
}
#[test]
fn gameobject_use_flagdrop_unknown_entry_keeps_delete_without_bg_click_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 27);
    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);

    assert!(session.use_represented_gameobject_flagdrop_like_cpp(
        gameobject_guid,
        player_guid,
        1,
        wow_entities::FlagDropUseSource {
            event_id: 0,
            pickup_spell_id: 33,
            expire_duration_ms: 44,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RemoveStealthOrInvisibilityAuras {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::BattlegroundFlagDropClicked {
                gameobject_guid,
                player_guid,
                gameobject_entry: 1,
                click_target: BattlegroundFlagDropClickTarget::None,
                event_id: 0,
                pickup_spell_id: 33,
                expire_duration_ms: 44,
            },
            RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid },
        ]
    );
}
#[test]
fn gameobject_use_new_flag_records_pickup_request_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 36);

    assert!(session.use_represented_gameobject_new_flag_like_cpp(
        gameobject_guid,
        player_guid,
        179_830,
        wow_entities::NewFlagUseSource {
            pickup_spell_id: 11,
            expire_duration_ms: 22,
            respawn_time_ms: 33,
            flag_drop_entry: 44,
            exclusive_category: -55,
            world_state_id: 66,
            return_on_defender_interact: true,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::NewFlagPickupRequested {
                gameobject_guid,
                player_guid,
                pickup_spell_id: 11,
                expire_duration_ms: 22,
                respawn_time_ms: 33,
                flag_drop_entry: 44,
                exclusive_category: -55,
                world_state_id: 66,
                return_on_defender_interact: true,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 179_830,
                spell_id: 11,
                go_type: wow_entities::GAMEOBJECT_TYPE_NEW_FLAG,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: gameobject_guid,
                spell_id: 11,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::GameObject,
                spell_lookup_difficulty_id: 0,
            },
            RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                gameobject_guid,
                player_guid,
                state: RepresentedNewFlagStateRequest::Taken,
            },
        ]
    );
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.new_flag_state,
        Some(RepresentedNewFlagStateRequest::Taken)
    );
    assert_eq!(state.new_flag_carrier_guid, Some(player_guid));
    assert!(state.new_flag_taken_from_base_game_time_ms.is_some());
    assert!(state.new_flag_respawn_until.is_none());
}
#[test]
fn gameobject_use_new_flag_rejects_non_in_base_state_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 36);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .new_flag_state = Some(RepresentedNewFlagStateRequest::Taken);

    assert!(!session.use_represented_gameobject_new_flag_like_cpp(
        gameobject_guid,
        player_guid,
        179_830,
        wow_entities::NewFlagUseSource {
            pickup_spell_id: 11,
            expire_duration_ms: 22,
            respawn_time_ms: 33,
            flag_drop_entry: 44,
            exclusive_category: -55,
            world_state_id: 66,
            return_on_defender_interact: true,
        },
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());
}
