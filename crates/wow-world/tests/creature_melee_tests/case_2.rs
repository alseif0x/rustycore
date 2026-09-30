use super::*;

#[tokio::test]
async fn apply_creature_melee_damage_command_syncs_health_without_visible_attacker_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1008);
    let victim_guid = ObjectGuid::create_player(1, 7002);
    session.fixture_melee_set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(victim_guid));
    session.fixture_melee_set_map_position(571, Position::ZERO);
    session.fixture_melee_set_health(100, 100);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        83,
        wow_constants::DeathState::Alive,
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 17,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 83,
                victim_health_state_revision_after: committed_revision,
                hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
                victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
                original_damage: 17,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: Vec::new(),
            },
        ))
        .expect("command queued");
    session
        .fixture_melee_process_commands()
        .await;

    assert_eq!(session.fixture_melee_health(), 83);
    let packet = send_rx
        .try_recv()
        .expect("authoritative health update is independent of attacker visibility");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
    assert!(send_rx.try_recv().is_err(), "no attacker-state packet");
}
