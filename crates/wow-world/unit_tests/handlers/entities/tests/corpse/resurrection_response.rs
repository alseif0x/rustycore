use super::*;

#[tokio::test]
async fn resurrect_response_accepts_matching_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    let resurrecter = ObjectGuid::create_player(1, 77);
    let player_guid = ObjectGuid::create_player(1, 42);
    let target_position = Position::new(11.0, 22.0, 33.0, 1.5);
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    crate::player_inventory_test_fixture::complete_known_empty_player_inventory_like_cpp(
        &mut session,
    );
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_player_health_like_cpp(0, 1_000);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Mana, Some(0));
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Rage, Some(1));
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Energy, Some(2));
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Focus, Some(3));
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Mana, 2_000);
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Rage, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Rage, 25);
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Energy, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Energy, 20);
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Focus, 80);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Focus, 13);
        })
        .unwrap();
    let request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter,
        map_id: 571,
        position: target_position,
        health: 450,
        mana: 120,
        aura: 0,
    };
    session.set_represented_resurrection_request_like_cpp(request);

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    let awaiting = wow_entities::PlayerResurrectionContinuation {
        operation: wow_entities::PlayerResurrectionOperation::RequestData(request),
        phase: wow_entities::PlayerResurrectionPhase::AwaitingTeleport,
    };
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session.represented_resurrection_request_like_cpp(),
        Some(request)
    );
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(awaiting)
    );
    assert_eq!(
        crate::session::hub_ref(&session).movement_counter_like_cpp(),
        Some(1)
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session.represented_resurrection_request_like_cpp(),
        Some(request)
    );
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(awaiting)
    );
    assert_eq!(
        crate::session::hub_ref(&session).movement_counter_like_cpp(),
        Some(2)
    );
    assert_eq!(
        send_rx.try_iter().collect::<Vec<_>>(),
        (0..=1)
            .map(|sequence_index| {
                wow_packet::packets::movement::MoveTeleport {
                    mover_guid: player_guid,
                    position: target_position,
                    facing: target_position.orientation,
                    sequence_index,
                    preload_world: 0,
                    transport_guid: Some(ObjectGuid::EMPTY),
                }
                .to_bytes()
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(realm_rx.len(), 2);

    let action =
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 1, 0);
    assert_eq!(
        action,
        crate::session::MoveTeleportAckActionLikeCpp::Accepted
    );
    assert!(wow_world_entities::mark_player_resurrection_teleport_completed(&mut session));
    let teleport_completed = wow_entities::PlayerResurrectionContinuation {
        phase: wow_entities::PlayerResurrectionPhase::TeleportCompleted,
        ..awaiting
    };
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(teleport_completed)
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(teleport_completed),
        "a post-ACK repeat cannot restart the completed teleport"
    );
    assert!(send_rx.is_empty());
    assert_eq!(realm_rx.len(), 2);

    let completed = continue_resurrection_after_valid_teleport(&mut session).await;
    assert!(
        completed,
        "resurrection continuation remains {:?}",
        session.represented_delayed_resurrection_after_teleport_like_cpp()
    );

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 450);
    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(player.unit().get_power(wow_constants::PowerType::Mana), 120);
            assert_eq!(player.unit().get_power(wow_constants::PowerType::Rage), 0);
            assert_eq!(
                player.unit().get_power(wow_constants::PowerType::Energy),
                100
            );
            assert_eq!(player.unit().get_power(wow_constants::PowerType::Focus), 80);
        })
        .unwrap();
    assert_eq!(session.player_position_like_cpp(), Some(target_position));
    assert!(
        session
            .represented_delayed_resurrection_after_teleport_like_cpp()
            .is_none()
    );
    let realm = realm_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(realm.len(), 7);
    assert_eq!(
        u16::from_le_bytes([realm[0][0], realm[0][1]]),
        ServerOpcodes::CancelCombat as u16
    );
    assert_eq!(
        u16::from_le_bytes([realm[1][0], realm[1][1]]),
        ServerOpcodes::CancelCombat as u16
    );
    assert_eq!(
        u16::from_le_bytes([realm[2][0], realm[2][1]]),
        ServerOpcodes::DeathReleaseLoc as u16
    );
    for (packet, (power, value)) in realm[3..].iter().zip([
        (wow_constants::PowerType::Mana, 120),
        (wow_constants::PowerType::Rage, 0),
        (wow_constants::PowerType::Energy, 100),
        (wow_constants::PowerType::Focus, 80),
    ]) {
        assert_eq!(
            *packet,
            wow_packet::packets::combat::PowerUpdate {
                guid: player_guid,
                powers: vec![(value, power as u8)],
            }
            .to_bytes()
        );
    }
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    assert!(
        wow_world_entities::continue_player_resurrection_after_teleport(
            &mut session,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creature_spawns,
                graveyards: None,
            },
        )
        .await
    );
    assert!(
        realm_rx.try_recv().is_err(),
        "a completed request cannot publish powers twice"
    );

    while send_rx.try_recv().is_ok() {}
    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;
    assert!(send_rx.is_empty());
    assert!(realm_rx.is_empty());
    assert_eq!(session.player_health_like_cpp(), 450);
}

#[tokio::test]
async fn resurrect_response_repeats_pre_ack_accept_without_canonical_owner_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let (realm_tx, _realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_player_health_like_cpp(0, 1_000);
    let resurrecter = ObjectGuid::create_player(1, 77);
    let request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter,
        map_id: 571,
        position: Position::new(11.0, 22.0, 33.0, 1.5),
        health: 450,
        mana: 120,
        aura: 0,
    };
    session.set_represented_resurrection_request_like_cpp(request);
    assert!(session.core.player_handle_like_cpp.is_none());

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;
    let awaiting = wow_entities::PlayerResurrectionContinuation {
        operation: wow_entities::PlayerResurrectionOperation::RequestData(request),
        phase: wow_entities::PlayerResurrectionPhase::AwaitingTeleport,
    };
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(awaiting)
    );
    let inconsistent = wow_entities::PlayerResurrectionContinuation {
        operation: wow_entities::PlayerResurrectionOperation::RequestData(
            wow_entities::PlayerResurrectionRequestLikeCpp {
                resurrecter: ObjectGuid::create_player(1, 78),
                ..request
            },
        ),
        phase: wow_entities::PlayerResurrectionPhase::AwaitingTeleport,
    };
    session
        .fixtures
        .combat
        .represented_delayed_resurrection_after_teleport_like_cpp = Some(inconsistent);

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert_eq!(
        session.represented_resurrection_request_like_cpp(),
        Some(request)
    );
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(inconsistent),
        "a continuation for another resurrecter is rejected without mutation"
    );
    session
        .fixtures
        .combat
        .represented_delayed_resurrection_after_teleport_like_cpp =
        Some(wow_entities::PlayerResurrectionContinuation {
            phase: wow_entities::PlayerResurrectionPhase::TeleportInFlight,
            ..awaiting
        });

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert_eq!(
        session.represented_resurrection_request_like_cpp(),
        Some(request)
    );
    assert_eq!(
        session.represented_delayed_resurrection_after_teleport_like_cpp(),
        Some(awaiting),
        "a subsequent matching accept advances the repeated teleport"
    );
}

#[tokio::test]
async fn resurrect_response_decline_clears_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 78);
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 1)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_ignores_mismatched_resurrecter_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 79);
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(
        &mut session,
        resurrect_response_packet(ObjectGuid::create_player(1, 80), 0),
    )
    .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_some()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_ignores_alive_player_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 81);
    session.set_player_health_like_cpp(777, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 777);
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_some()
    );
    assert!(send_rx.try_recv().is_err());
}
