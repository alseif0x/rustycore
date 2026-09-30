use super::*;

#[tokio::test]
async fn apply_creature_melee_damage_command_delayed_after_heal_presents_current_canonical_state_like_cpp()
 {
    let (mut session, _, send_rx) = make_session();
    let attacker_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1014);
    let victim_guid = ObjectGuid::create_player(1, 7008);
    session.fixture_melee_set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(victim_guid));
    session.fixture_melee_set_map_position(571, Position::ZERO);
    session.fixture_melee_set_health(83, 100);
    session.fixture_melee_make_visible(attacker_guid);
    let committed_revision = install_committed_canonical_player_health_for_melee_test_like_cpp(
        &mut session,
        victim_guid,
        83,
        wow_constants::DeathState::Alive,
    );
    let command = ApplyCreatureMeleeDamageLikeCppCommand {
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
    };

    session
        .fixture_melee_mutate_player(|player| player.unit_mut().set_health(95))
        .expect("canonical player remains present for delayed delivery");
    session.fixture_melee_set_health(95, 100);
    let canonical_before = session
        .fixture_melee_mutate_player(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
        .expect("delayed command queued");

    session.fixture_melee_process_commands().await;

    let canonical_after = session
        .fixture_melee_mutate_player(|player| {
            (
                player.unit().data().health,
                player.unit().death_state(),
                player.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(canonical_after, canonical_before);
    assert_eq!(session.fixture_melee_health(), 95);
    assert!(session.fixture_melee_is_alive());
    assert_eq!(
        session
            .view
            .last_presented_creature_melee_health_state_revision_like_cpp,
        committed_revision
    );

    let attacker_state = send_rx.try_recv().expect("committed hit is presented once");
    assert_eq!(
        u16::from_le_bytes([attacker_state[0], attacker_state[1]]),
        ServerOpcodes::AttackerStateUpdate as u16
    );
    let health_update = send_rx.try_recv().expect("current health is presented");
    let mut health_update = wow_packet::world_packet::WorldPacket::from_bytes(&health_update);
    assert_eq!(
        health_update.read_uint16().unwrap(),
        ServerOpcodes::HealthUpdate as u16
    );
    assert_eq!(health_update.read_packed_guid().unwrap(), victim_guid);
    assert_eq!(health_update.read_int64().unwrap(), 95);
    assert!(send_rx.try_recv().is_err());
}
