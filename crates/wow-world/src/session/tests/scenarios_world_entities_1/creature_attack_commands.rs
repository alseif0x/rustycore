use super::*;

#[tokio::test]
async fn send_if_visible_creature_command_rechecks_current_phase_and_range_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1011);
    let packet_bytes = vec![0xD4, 0x2D, 0xAA];
    let manager = shared_map_manager();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::new(5000.0, 5000.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    session.state = SessionState::LoggedIn;
    session.set_map_manager(Arc::clone(&manager));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects stale HaveAtClient entries outside source range"
    );

    session.mutate_world_creature(source_guid, |creature| {
        creature
            .creature
            .set_ai_position(Position::new(10.0, 0.0, 0.0, 0.0));
        *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
    });
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));
    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects stale HaveAtClient entries outside source phase"
    );

    session.mutate_world_creature(source_guid, |creature| {
        *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    });
    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(
        send_rx.try_recv().expect("same phase and range delivers"),
        packet_bytes
    );
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}

/// Future global creature aggro must mirror one map-owned AttackStart into
/// the victim session. C++ anchor: `CreatureAI::MoveInLineOfSight` ->
/// `Creature::CanStartAttack` -> `Unit::SendMeleeAttackStart`.
#[tokio::test]
async fn creature_attack_start_command_sets_combat_and_sends_packet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1006);
    let victim_guid = ObjectGuid::create_player(1, 7000);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(attacker_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::CreatureAttackStartLikeCpp(
            CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        session.combat_target, None,
        "incoming attacks must not select the attacker as the player's target"
    );
    assert!(session.in_combat);
    let packet = send_rx.try_recv().expect("attack start packet");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackStart as u16);
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}

#[tokio::test]
async fn creature_attack_start_command_syncs_combat_when_attacker_is_no_longer_visible_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1009);
    let victim_guid = ObjectGuid::create_player(1, 7003);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::CreatureAttackStartLikeCpp(
            CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        session.combat_target, None,
        "incoming attacks must not select the attacker as the player's target"
    );
    assert!(session.in_combat);
    assert!(
        send_rx.try_recv().is_err(),
        "an attacker no longer visible to the client must not emit AttackStart"
    );
}

#[tokio::test]
async fn creature_attack_start_command_rejects_dead_victim_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1010);
    let victim_guid = ObjectGuid::create_player(1, 7004);
    session.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(attacker_guid);
    session.set_player_alive_like_cpp(false);

    session
        .session_command_tx()
        .try_send(SessionCommand::CreatureAttackStartLikeCpp(
            CreatureAttackStartLikeCppCommand {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
    assert!(
        send_rx.try_recv().is_err(),
        "dead victim must not receive attack-start"
    );
}
