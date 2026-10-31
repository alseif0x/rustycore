use super::*;

#[tokio::test]
async fn durable_creature_runtime_rail_is_drained_by_session_update_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1011);
    let victim_guid = ObjectGuid::create_player(1, 7005);
    session.core.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(100, 100);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        95,
        wow_constants::DeathState::Alive,
    );
    assert!(
        session
            .core
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap()
            .publish_melee_damage_like_cpp(ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 5,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 95,
                victim_health_state_revision_after: committed_revision,
                hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
                victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
                original_damage: 5,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: Vec::new(),
            })
    );

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.player_health_like_cpp(), 95);
    assert!(session.player_is_alive_like_cpp());
    let health_packet = send_rx
        .try_recv()
        .expect("the committed nonlethal hit presents its health update");
    let mut health_update = wow_packet::world_packet::WorldPacket::from_bytes(&health_packet);
    assert_eq!(
        health_update.read_uint16().unwrap(),
        ServerOpcodes::HealthUpdate as u16
    );
    assert_eq!(health_update.read_packed_guid().unwrap(), victim_guid);
    assert_eq!(health_update.read_int64().unwrap(), 95);
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}

#[tokio::test]
async fn durable_creature_runtime_overflow_disconnects_desynchronized_session() {
    let (mut session, _, _) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1012);
    let victim_guid = ObjectGuid::create_player(1, 7006);
    session.core.state = SessionState::LoggedIn;
    session.set_player_guid(Some(victim_guid));
    let command = CreatureAttackStartLikeCppCommand {
        attacker_guid,
        victim_guid,
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 0,
        packet_already_broadcast: false,
    };
    {
        let mut pending = session
            .core
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap();
        for _ in 0..crate::session::mailbox::MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            assert!(pending.publish_attack_start_like_cpp(command.clone()));
        }
        assert!(!pending.publish_attack_start_like_cpp(command));
    }

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(session.is_disconnecting());
    assert!(
        session
            .core
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap()
            .drain_like_cpp()
            .is_empty()
    );
}
